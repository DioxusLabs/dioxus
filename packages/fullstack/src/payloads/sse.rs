use crate::{ClientResponse, FromResponse, RequestError, ServerFnError};
#[cfg(feature = "server")]
use axum::{
    response::sse::{Event, KeepAlive},
    BoxError,
};
use futures::io::AsyncBufReadExt;
use futures::Stream;
use futures::{StreamExt, TryStreamExt};
use http::{header::CONTENT_TYPE, HeaderValue, StatusCode};
use serde::de::DeserializeOwned;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

/// A stream of Server-Sent Events (SSE) that can be used to receive events from the server.
///
/// This type implements `Stream` for asynchronous iteration over events.
/// Events are automatically deserialized from JSON to the specified type `T`.
#[allow(clippy::type_complexity)]
pub struct ServerEvents<T> {
    _marker: std::marker::PhantomData<fn() -> T>,

    // The receiving end from the server
    client: Option<Pin<Box<dyn Stream<Item = Result<ServerSentEvent, ServerFnError>>>>>,

    #[cfg(feature = "server")]
    keep_alive: Option<KeepAlive>,

    // The actual SSE response to send to the client
    #[cfg(feature = "server")]
    sse: Option<axum::response::Sse<Pin<Box<dyn Stream<Item = Result<Event, BoxError>> + Send>>>>,
}

impl<T: DeserializeOwned> ServerEvents<T> {
    /// Receives the next event from the stream, deserializing it to `T`.
    ///
    /// Returns `None` if the stream has ended.
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
    /// Receives the next raw event from the stream.
    ///
    /// Returns `None` if the stream has ended.
    pub async fn next_event(&mut self) -> Option<Result<ServerSentEvent, ServerFnError>> {
        self.client.as_mut()?.next().await
    }
}

impl<T: DeserializeOwned> Stream for ServerEvents<T> {
    type Item = Result<T, ServerFnError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let Some(client) = self.client.as_mut() else {
            return Poll::Ready(None);
        };

        match client.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(event))) => {
                let data = serde_json::from_str(&event.data).map_err(|err| {
                    ServerFnError::Serialization(format!(
                        "failed to deserialize event data: {}",
                        err
                    ))
                });
                Poll::Ready(Some(data))
            }
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(err))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T> FromResponse for ServerEvents<T> {
    async fn from_response(res: ClientResponse) -> Result<Self, ServerFnError> {
        let status = res.status();
        if status != StatusCode::OK {
            return Err(ServerFnError::Request(RequestError::Status(
                format!("Expected status 200 OK, got {}", status),
                status.as_u16(),
            )));
        }

        let content_type = res.headers().get(CONTENT_TYPE);
        if content_type != Some(&HeaderValue::from_static(mime::TEXT_EVENT_STREAM.as_ref())) {
            return Err(ServerFnError::Request(RequestError::Request(format!(
                "Expected content type 'text/event-stream', got {:?}",
                content_type
            ))));
        }

        let mut stream = res
            .bytes_stream()
            .map(|result| result.map_err(std::io::Error::other))
            .into_async_read();

        let mut line_buffer = String::new();
        let mut event_buffer = EventBuffer::new();

        let stream: Pin<Box<dyn Stream<Item = Result<ServerSentEvent, ServerFnError>>>> = Box::pin(
            async_stream::try_stream! {
                loop {
                    line_buffer.clear();
                    if stream.read_line(&mut line_buffer).await.map_err(|err| ServerFnError::StreamError(err.to_string()))? == 0 {
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

                    // Parse line to split field name and value, applying proper trimming.
                    let (field, value) = line.split_once(':').unwrap_or((line, ""));
                    let value = value.strip_prefix(' ').unwrap_or(value);

                    // Handle fields - these are the in SSE speci.
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
            },
        );

        Ok(Self {
            _marker: std::marker::PhantomData,
            client: Some(stream),

            #[cfg(feature = "server")]
            keep_alive: None,

            #[cfg(feature = "server")]
            sse: None,
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

/// Internal buffer used to accumulate lines of an SSE (Server-Sent Events) stream.
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

    /// Set the [`Event`]'s type. Override previous value.
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

#[cfg(feature = "server")]
pub use server_impl::*;

#[cfg(feature = "server")]
mod server_impl {
    use super::*;
    use crate::spawn_platform;
    use axum::response::sse::Sse;
    use axum_core::response::IntoResponse;
    use futures::Future;
    use futures::SinkExt;
    use futures::{Sink, TryStream};
    use serde::Serialize;

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

            let sse = Sse::new(stream.boxed());

            Self {
                _marker: std::marker::PhantomData,
                client: None,
                keep_alive: Some(KeepAlive::new().interval(Duration::from_secs(15))),
                sse: Some(sse),
            }
        }

        /// Create a `ServerEvents` from a `TryStream` of events.
        pub fn from_stream<S>(stream: S) -> Self
        where
            S: TryStream<Ok = T, Error = BoxError> + Send + 'static,
            T: Serialize,
        {
            let stream = stream.map_ok(|event| {
                axum::response::sse::Event::default()
                    .json_data(event)
                    .expect("Failed to serialize SSE event")
            });
            let sse = axum::response::Sse::new(stream.boxed());
            Self {
                _marker: std::marker::PhantomData,
                client: None,
                keep_alive: Some(KeepAlive::new().interval(Duration::from_secs(15))),
                sse: Some(sse),
            }
        }

        /// Set the keep-alive configuration for the SSE connection.
        ///
        /// A `None` value will disable the default `KeepAlive` of 15 seconds.
        pub fn with_keep_alive(mut self, keep_alive: Option<KeepAlive>) -> Self {
            self.keep_alive = keep_alive;
            self
        }

        /// Create a `ServerEvents` from an existing Axum `Sse` response.
        #[allow(clippy::type_complexity)]
        pub fn from_sse(
            sse: Sse<Pin<Box<dyn Stream<Item = Result<Event, BoxError>> + Send>>>,
        ) -> Self {
            Self {
                _marker: std::marker::PhantomData,
                client: None,
                keep_alive: None,
                sse: Some(sse),
            }
        }
    }

    impl<T> IntoResponse for ServerEvents<T> {
        fn into_response(self) -> axum_core::response::Response {
            let sse = self
                .sse
                .expect("SSE should be initialized before using it as a response");

            if let Some(keep_alive) = self.keep_alive {
                sse.keep_alive(keep_alive).into_response()
            } else {
                sse.into_response()
            }
        }
    }

    /// A transmitter for sending events to the SSE stream.
    pub struct SseTx<T> {
        sender: futures_channel::mpsc::UnboundedSender<axum::response::sse::Event>,
        _marker: std::marker::PhantomData<fn() -> T>,
    }

    impl<T: Serialize> SseTx<T> {
        /// Sends an event to the SSE stream.
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

    impl<T: Serialize> Sink<T> for SseTx<T> {
        type Error = anyhow::Error;

        fn poll_ready(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            self.sender.poll_ready_unpin(_cx).map_err(|e| e.into())
        }

        fn start_send(mut self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
            let event = axum::response::sse::Event::default().json_data(item)?;
            self.sender.start_send(event).map_err(|e| e.into())
        }

        fn poll_flush(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            self.sender.poll_flush_unpin(_cx).map_err(|e| e.into())
        }

        fn poll_close(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            self.sender.poll_close_unpin(_cx).map_err(|e| e.into())
        }
    }
}
