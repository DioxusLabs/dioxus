#![allow(clippy::type_complexity)]

use crate::{
    CborEncoding, ClientRequest, ClientResponse, Encoding, FromResponse, IntoRequest, JsonEncoding,
    ServerFnError,
};
use axum::extract::{FromRequest, Request};
use axum_core::response::IntoResponse;
use bytes::Bytes;
use dioxus_fullstack_core::{HttpError, RequestError};
use futures::{Stream, StreamExt};
use headers::{ContentType, Header};
use send_wrapper::SendWrapper;
use serde::{de::DeserializeOwned, Serialize};
use std::{future::Future, marker::PhantomData, pin::Pin};

pub type TextStream = Streaming<String>;
pub type ByteStream = Streaming<Bytes>;
pub type JsonStream<T> = Streaming<T, JsonEncoding>;
pub type CborStream<T> = Streaming<T, CborEncoding>;

/// A streaming payload.
///
/// ## Browser Support for Streaming Input
///
/// Browser fetch requests do not currently support full request duplexing, which
/// means that that they do not begin handling responses until the full request has been sent.
///
/// This means that if you use a streaming input encoding, the input stream needs to
/// end before the output will begin.
///
/// Streaming requests are only allowed over HTTP2 or HTTP3.
///
/// Also note that not all browsers support streaming bodies to servers.
pub struct Streaming<T = String, E = ()> {
    output_stream: Pin<Box<dyn Stream<Item = Result<T, StreamingError>> + Send>>,
    input_stream: Pin<Box<dyn Stream<Item = Result<T, StreamingError>> + Send>>,
    encoding: PhantomData<E>,
}

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq, Hash)]
pub enum StreamingError {
    /// The streaming request was interrupted and could not be completed.
    #[error("The streaming request was interrupted")]
    Interrupted,

    /// The stream failed to decode a chunk - possibly due to invalid data or version mismatch.
    #[error("The stream failed to decode a chunk")]
    Decoding,

    /// The stream failed to connect or encountered an error.
    #[error("The streaming request failed")]
    Failed,
}

impl<T: 'static + Send, E> Streaming<T, E> {
    /// Creates a new stream from the given stream.
    pub fn new(value: impl Stream<Item = T> + Send + 'static) -> Self {
        // Box and pin the incoming stream and store as a trait object
        Self {
            output_stream: Box::pin(futures::stream::empty()) as _,
            input_stream: Box::pin(value.map(|item| Ok(item)))
                as Pin<Box<dyn Stream<Item = Result<T, StreamingError>> + Send>>,
            encoding: PhantomData,
        }
    }

    /// Spawns a new task that produces items for the stream.
    ///
    /// The callback is provided an `UnboundedSender` that can be used to send items to the stream.
    #[cfg(feature = "server")]
    pub fn spawn<F>(
        callback: impl FnOnce(futures_channel::mpsc::UnboundedSender<T>) -> F + Send + 'static,
    ) -> Self
    where
        F: Future<Output = ()> + 'static,
        T: Send,
    {
        let (tx, rx) = futures_channel::mpsc::unbounded();

        crate::spawn_platform(move || callback(tx));

        Self::new(rx)
    }

    /// Returns the next item in the stream, or `None` if the stream has ended.
    pub async fn next(&mut self) -> Option<Result<T, StreamingError>> {
        self.output_stream.as_mut().next().await
    }

    /// Consumes the wrapper, returning the inner stream.
    pub fn into_inner(self) -> impl Stream<Item = Result<T, StreamingError>> + Send {
        self.input_stream
    }
}

impl<S, U> From<S> for TextStream
where
    S: Stream<Item = U> + Send + 'static,
    U: Into<String>,
{
    fn from(value: S) -> Self {
        Self {
            input_stream: Box::pin(value.map(|data| Ok(data.into()))),
            output_stream: Box::pin(futures::stream::empty()) as _,
            encoding: PhantomData,
        }
    }
}

impl<S, E> From<S> for ByteStream
where
    S: Stream<Item = Result<Bytes, E>> + Send + 'static,
{
    fn from(value: S) -> Self {
        Self {
            input_stream: Box::pin(value.map(|data| data.map_err(|_| StreamingError::Failed))),
            output_stream: Box::pin(futures::stream::empty()) as _,
            encoding: PhantomData,
        }
    }
}

impl<T, S, U, E> From<S> for Streaming<T, E>
where
    S: Stream<Item = U> + Send + 'static,
    U: Into<T>,
    T: 'static + Send,
    E: Encoding,
{
    fn from(value: S) -> Self {
        Self {
            input_stream: Box::pin(value.map(|data| Ok(data.into()))),
            output_stream: Box::pin(futures::stream::empty()) as _,
            encoding: PhantomData,
        }
    }
}

impl IntoResponse for Streaming<String> {
    fn into_response(self) -> axum_core::response::Response {
        axum::response::Response::builder()
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(axum::body::Body::from_stream(self.input_stream))
            .unwrap()
    }
}

impl IntoResponse for Streaming<Bytes> {
    fn into_response(self) -> axum_core::response::Response {
        axum::response::Response::builder()
            .header("Content-Type", "application/octet-stream")
            .body(axum::body::Body::from_stream(self.input_stream))
            .unwrap()
    }
}

impl<T: DeserializeOwned + Serialize + 'static, E: Encoding> IntoResponse for Streaming<T, E> {
    fn into_response(self) -> axum_core::response::Response {
        let res = self.input_stream.map(|r| match r {
            Ok(res) => match E::to_bytes(&res) {
                Some(bytes) => Ok(bytes),
                None => Err(StreamingError::Failed),
            },
            Err(_err) => Err(StreamingError::Failed),
        });

        axum::response::Response::builder()
            .header("Content-Type", E::stream_content_type())
            .body(axum::body::Body::from_stream(res))
            .unwrap()
    }
}

impl FromResponse for Streaming<String> {
    fn from_response(res: ClientResponse) -> impl Future<Output = Result<Self, ServerFnError>> {
        SendWrapper::new(async move {
            let client_stream = Box::pin(res.bytes_stream().map(|byte| match byte {
                Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
                    Ok(string) => Ok(string),
                    Err(_) => Err(StreamingError::Decoding),
                },
                Err(_) => Err(StreamingError::Failed),
            }));

            Ok(Self {
                output_stream: client_stream,
                input_stream: Box::pin(futures::stream::empty()),
                encoding: PhantomData,
            })
        })
    }
}

impl FromResponse for Streaming<Bytes> {
    fn from_response(res: ClientResponse) -> impl Future<Output = Result<Self, ServerFnError>> {
        async move {
            let client_stream = Box::pin(SendWrapper::new(res.bytes_stream().map(
                |byte| match byte {
                    Ok(bytes) => Ok(bytes),
                    Err(_) => Err(StreamingError::Failed),
                },
            )));

            Ok(Self {
                output_stream: client_stream,
                input_stream: Box::pin(futures::stream::empty()),
                encoding: PhantomData,
            })
        }
    }
}

impl<T: DeserializeOwned + Serialize + 'static + Send, E: Encoding> FromResponse
    for Streaming<T, E>
{
    fn from_response(res: ClientResponse) -> impl Future<Output = Result<Self, ServerFnError>> {
        SendWrapper::new(async move {
            let client_stream = Box::pin(SendWrapper::new(res.bytes_stream().map(
                |byte| match byte {
                    Ok(bytes) => match E::from_bytes(bytes) {
                        Some(res) => Ok(res),
                        None => Err(StreamingError::Decoding),
                    },
                    Err(_) => Err(StreamingError::Failed),
                },
            )));

            Ok(Self {
                output_stream: client_stream,
                input_stream: Box::pin(futures::stream::empty()),
                encoding: PhantomData,
            })
        })
    }
}

impl<S> FromRequest<S> for Streaming<String> {
    type Rejection = ServerFnError;

    fn from_request(
        req: Request,
        _state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let (parts, body) = req.into_parts();
            let content_type = parts
                .headers
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            if !content_type.starts_with("text/plain") {
                HttpError::bad_request("Invalid content type")?;
            }

            let stream = body.into_data_stream();

            Ok(Self {
                input_stream: Box::pin(futures::stream::empty()),
                output_stream: Box::pin(stream.map(|byte| match byte {
                    Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
                        Ok(string) => Ok(string),
                        Err(_) => Err(StreamingError::Decoding),
                    },
                    Err(_) => Err(StreamingError::Failed),
                })),
                encoding: PhantomData,
            })
        }
    }
}

impl<S> FromRequest<S> for ByteStream {
    type Rejection = ServerFnError;

    fn from_request(
        req: Request,
        _state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let (parts, body) = req.into_parts();
            let content_type = parts
                .headers
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            if !content_type.starts_with("application/octet-stream") {
                HttpError::bad_request("Invalid content type")?;
            }

            let stream = body.into_data_stream();

            Ok(Self {
                input_stream: Box::pin(futures::stream::empty()),
                output_stream: Box::pin(stream.map(|byte| match byte {
                    Ok(bytes) => Ok(bytes),
                    Err(_) => Err(StreamingError::Failed),
                })),
                encoding: PhantomData,
            })
        }
    }
}

impl<T: DeserializeOwned + Serialize + 'static + Send, E: Encoding, S> FromRequest<S>
    for Streaming<T, E>
{
    type Rejection = ServerFnError;

    fn from_request(
        req: Request,
        _state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let (parts, body) = req.into_parts();
            let content_type = parts
                .headers
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            if !content_type.starts_with(E::stream_content_type()) {
                HttpError::bad_request("Invalid content type")?;
            }

            let stream = body.into_data_stream();

            Ok(Self {
                input_stream: Box::pin(futures::stream::empty()),
                output_stream: Box::pin(stream.map(|byte| match byte {
                    Ok(bytes) => match E::from_bytes(bytes) {
                        Some(res) => Ok(res),
                        None => Err(StreamingError::Decoding),
                    },
                    Err(_) => Err(StreamingError::Failed),
                })),
                encoding: PhantomData,
            })
        }
    }
}

impl IntoRequest for Streaming<String> {
    fn into_request(
        self,
        builder: ClientRequest,
    ) -> impl Future<Output = Result<ClientResponse, RequestError>> + 'static {
        async move {
            builder
                .header("Content-Type", "text/plain; charset=utf-8")?
                .send_body_stream(self.input_stream.map(|e| e.map(Bytes::from)))
                .await
        }
    }
}

impl IntoRequest for ByteStream {
    fn into_request(
        self,
        builder: ClientRequest,
    ) -> impl Future<Output = Result<ClientResponse, RequestError>> + 'static {
        async move {
            builder
                .header(ContentType::name(), "application/octet-stream")?
                .send_body_stream(self.input_stream)
                .await
        }
    }
}

impl<T: DeserializeOwned + Serialize + 'static + Send, E: Encoding> IntoRequest
    for Streaming<T, E>
{
    fn into_request(
        self,
        builder: ClientRequest,
    ) -> impl Future<Output = Result<ClientResponse, RequestError>> + 'static {
        async move {
            builder
                .header("Content-Type", E::stream_content_type())?
                .send_body_stream(
                    self.input_stream.map(|r| {
                        r.and_then(|item| E::to_bytes(&item).ok_or(StreamingError::Failed))
                    }),
                )
                .await
        }
    }
}

impl<T> std::fmt::Debug for Streaming<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Streaming").finish()
    }
}

impl<T, E: Encoding> std::fmt::Debug for Streaming<T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Streaming")
            .field("encoding", &std::any::type_name::<E>())
            .finish()
    }
}
