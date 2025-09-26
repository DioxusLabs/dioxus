use std::pin::Pin;
use std::{future::Future, time::Duration};

use async_stream::try_stream;
#[cfg(feature = "server")]
use axum::{
    response::sse::{Event, KeepAlive},
    BoxError,
};

use axum_core::response::IntoResponse;
use futures::{Stream, TryStream};
use futures::{StreamExt, TryStreamExt};
use http::{header::CONTENT_TYPE, StatusCode};
use reqwest::header::HeaderValue;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt::Display;

use crate::{ClientResponse, FromResponse, ServerFnError};

pub struct ServerEvents<T> {
    _marker: std::marker::PhantomData<fn() -> T>,

    // The receiving end from the server
    client: Option<Pin<Box<dyn Stream<Item = Result<ServerSentEvent, ServerFnError>>>>>,

    // The actual SSE response to send to the client
    #[cfg(feature = "server")]
    sse: Option<axum::response::Sse<Pin<Box<dyn Stream<Item = Result<Event, BoxError>> + Send>>>>,
}

impl<T: DeserializeOwned> ServerEvents<T> {
    pub async fn recv(&mut self) -> Option<Result<T, ServerFnError>> {
        let event = self.next_event().await?;
        match event {
            Ok(event) => {
                let data: Result<T, ServerFnError> =
                    serde_json::from_str(&event.data).map_err(|err| {
                        ServerFnError::Serialization(format!(
                            "failed to deserialize event data: {}",
                            err
                        ))
                    });
                Some(data)
            }
            Err(err) => Some(Err(err)),
        }
    }
}

impl<T> ServerEvents<T> {
    pub async fn next_event(&mut self) -> Option<Result<ServerSentEvent, ServerFnError>> {
        let client = self.client.as_mut()?;
        let res = client.next().await;
        res
    }
}

impl<T> FromResponse for ServerEvents<T> {
    fn from_response(
        res: ClientResponse,
    ) -> impl Future<Output = Result<Self, ServerFnError>> + Send {
        use futures::io::AsyncBufReadExt;

        let res = res;
        send_wrapper::SendWrapper::new(async move {
            let status = res.status();
            if status != StatusCode::OK {
                todo!()
                // return Err(EventSourceError::BadStatus(status));
            }
            let content_type = res.headers().get(CONTENT_TYPE);
            if content_type != Some(&HeaderValue::from_static(mime::TEXT_EVENT_STREAM.as_ref())) {
                todo!()
                // return Err(EventSourceError::BadContentType(content_type.cloned()));
            }

            let mut stream = res
                .bytes_stream()
                .map(|result| result.map_err(std::io::Error::other))
                .into_async_read();

            let mut line_buffer = String::new();
            let mut event_buffer = EventBuffer::new();

            let stream: Pin<Box<dyn Stream<Item = Result<ServerSentEvent, ServerFnError>>>> =
                Box::pin(try_stream! {
                    loop {
                        line_buffer.clear();
                        let count = stream.read_line(&mut line_buffer).await.map_err(|err| ServerFnError::StreamError(err.to_string()))?;
                        if count == 0 {
                            break;
                        }
                        let line = if let Some(line) = line_buffer.strip_suffix('\n') {
                            line
                        } else {
                            &line_buffer
                        };

                        // dispatch
                        if line.is_empty() {
                            if let Some(event) = event_buffer.produce_event() {
                                yield event;
                            }
                            continue;
                        }

                        let (field, value) = parse_line(line);

                        match field {
                            "event" => event_buffer.set_event_type(value),
                            "data" => event_buffer.push_data(value),
                            "id" => event_buffer.set_id(value),
                            "retry" => {
                                if let Ok(millis) = value.parse() {
                                    event_buffer.set_retry(Duration::from_millis(millis));
                                }
                            }
                            _ => {}
                        }
                    }
                });

            Ok(Self {
                _marker: std::marker::PhantomData,
                client: Some(stream),

                #[cfg(feature = "server")]
                sse: None,
            })
        })
    }
}

/// Server-Sent Event representation.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ServerSentEvent {
    /// A string identifying the type of event described.
    pub event_type: String,
    /// The data field for the message.
    pub data: String,
    /// Last event ID value.
    pub last_event_id: Option<String>,
    /// Reconnection time.
    pub retry: Option<Duration>,
}

#[cfg(feature = "server")]
pub use server_impl::*;

#[cfg(feature = "server")]
mod server_impl {
    use crate::spawn_platform;

    use super::*;

    pub struct SseTx<T> {
        sender: futures_channel::mpsc::UnboundedSender<axum::response::sse::Event>,
        _marker: std::marker::PhantomData<fn() -> T>,
    }

    impl<T: Serialize> SseTx<T> {
        pub async fn send(&mut self, event: T) -> anyhow::Result<()> {
            let event = axum::response::sse::Event::default().json_data(event)?;
            self.sender.unbounded_send(event)?;
            Ok(())
        }
    }

    impl<T> std::ops::Deref for SseTx<T> {
        type Target = futures_channel::mpsc::UnboundedSender<axum::response::sse::Event>;
        fn deref(&self) -> &Self::Target {
            &self.sender
        }
    }

    impl<T> std::ops::DerefMut for SseTx<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.sender
        }
    }

    // Server impl....
    impl<T: 'static> ServerEvents<T> {
        /// Create a `ServerEvents` from a function that is given a sender to send events to the client.
        ///
        /// By default, we send a comment every 15 seconds to keep the connection alive.
        pub fn new<F, R>(f: impl FnOnce(SseTx<T>) -> F + Send + 'static) -> Self
        where
            F: Future<Output = R> + 'static,
            R: 'static + Send,
        {
            let (tx, mut rx) = futures_channel::mpsc::unbounded();

            let tx = SseTx {
                sender: tx,
                _marker: std::marker::PhantomData,
            };

            // Spawn the user function in the background
            spawn_platform(move || f(tx));

            // Create the stream of events, mapping the incoming events to `Ok`
            // If the user function ends, the stream will end and the connection will be closed
            let stream = futures::stream::poll_fn(move |cx| match rx.poll_next_unpin(cx) {
                std::task::Poll::Ready(Some(event)) => std::task::Poll::Ready(Some(
                    Ok(event) as Result<axum::response::sse::Event, BoxError>
                )),
                std::task::Poll::Ready(None) => std::task::Poll::Ready(None),
                std::task::Poll::Pending => std::task::Poll::Pending,
            });

            let sse = axum::response::sse::Sse::new(stream.boxed()).keep_alive(KeepAlive::new());

            Self {
                _marker: std::marker::PhantomData,
                client: None,
                sse: Some(sse),
            }
        }

        /// Create a `ServerEvents` from a `TryStream` of events.
        pub fn from_stream(
            _stream: impl TryStream<Ok = T, Error = BoxError> + Send + 'static,
        ) -> Self {
            todo!()
        }

        /// Create a `ServerEvents` from an existing Axum `Sse` response.
        pub fn from_sse<S>(_sse: axum::response::sse::Sse<S>) -> Self {
            todo!()
        }

        /// Set the keep-alive time for the SSE connection.
        ///
        /// This will send a comment every `duration` to keep the connection alive.
        pub fn with_keep_alive_time(self, duration: std::time::Duration) -> Self {
            todo!()
        }

        pub fn with_keep_alive(self, _keep_alive: axum::response::sse::KeepAlive) -> Self {
            todo!()
        }

        pub fn keep_alive(self, _keep_alive: axum::response::sse::KeepAlive) -> Self {
            todo!()
        }
    }

    #[cfg(feature = "server")]
    impl<T> IntoResponse for ServerEvents<T> {
        fn into_response(self) -> axum_core::response::Response {
            self.sse.unwrap().into_response()
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum EventSourceError {
    BadStatus(StatusCode),
    BadContentType(Option<HeaderValue>),
}

impl Display for EventSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventSourceError::BadStatus(status_code) => {
                write!(f, "expecting status code `200`, found: {status_code}")
            }
            EventSourceError::BadContentType(None) => {
                write!(
                    f,
                    "expecting \"text/event-stream\" content type, found none"
                )
            }
            EventSourceError::BadContentType(Some(header_value)) => {
                let content_type = header_value.to_str();
                match content_type {
                    Ok(content_type) => {
                        write!(
                            f,
                            "expecting \"text/event-stream\", found: \"{content_type}\"",
                        )
                    }
                    Err(_) => {
                        write!(f, "expecting \"text/event-stream\", found invalid value")
                    }
                }
            }
        }
    }
}

/// Internal buffer used to accumulate lines of an SSE (Server-Sent Events) stream.
///
/// A single [`EventBuffer`] can be used to process the whole stream. [`set_event_type`] and [`push_data`]
/// methods update the state. [`produce_event`] produces a proper [`Event`] and prepares the internal
/// state to process further data.
struct EventBuffer {
    event_type: String,
    data: String,
    last_event_id: Option<String>,
    retry: Option<Duration>,
}

impl EventBuffer {
    /// Creates fresh new [`EventBuffer`].
    #[allow(clippy::new_without_default)]
    fn new() -> Self {
        Self {
            event_type: String::new(),
            data: String::new(),
            last_event_id: None,
            retry: None,
        }
    }

    /// Produces a [`Event`], if current state allow it.
    ///
    /// Reset the internal state to process further data.
    fn produce_event(&mut self) -> Option<ServerSentEvent> {
        let event = if self.data.is_empty() {
            None
        } else {
            Some(ServerSentEvent {
                event_type: if self.event_type.is_empty() {
                    "message".to_string()
                } else {
                    self.event_type.clone()
                },
                data: self.data.to_string(),
                last_event_id: self.last_event_id.clone(),
                retry: self.retry,
            })
        };

        self.event_type.clear();
        self.data.clear();

        event
    }

    /// Set the [`Event`]'s type. Overide previous value.
    fn set_event_type(&mut self, event_type: &str) {
        self.event_type.clear();
        self.event_type.push_str(event_type);
    }

    /// Extends internal data with given data.
    fn push_data(&mut self, data: &str) {
        if !self.data.is_empty() {
            self.data.push('\n');
        }
        self.data.push_str(data);
    }

    fn set_id(&mut self, id: &str) {
        self.last_event_id = Some(id.to_string());
    }

    fn set_retry(&mut self, retry: Duration) {
        self.retry = Some(retry);
    }
}

/// Parse line to split field name and value, applying proper trimming.
fn parse_line(line: &str) -> (&str, &str) {
    let (field, value) = line.split_once(':').unwrap_or((line, ""));
    let value = value.strip_prefix(' ').unwrap_or(value);
    (field, value)
}
