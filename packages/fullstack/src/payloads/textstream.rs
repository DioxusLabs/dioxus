use crate::{CborEncoding, ClientResponse, Encoding, FromResponse, JsonEncoding, ServerFnError};
use axum_core::response::IntoResponse;
use bytes::Bytes;
use futures::{Stream, StreamExt};
use send_wrapper::SendWrapper;
use serde::{de::DeserializeOwned, Serialize};
use std::{future::Future, marker::PhantomData, pin::Pin};

pub type TextStream = Streaming<String>;
pub type ByteStream = Streaming<Bytes>;
pub type JsonStream<T> = Streaming<T, JsonEncoding>;
pub type CborStream<T> = Streaming<T, CborEncoding>;

/// A stream of text.
///
/// A server function can return this type if its output encoding is [`StreamingText`].
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
pub struct Streaming<T = String, E = ()> {
    client_stream: Option<Pin<Box<dyn Stream<Item = Result<T, StreamingError>>>>>,
    server_stream: Pin<Box<dyn Stream<Item = T> + Send>>,
    encoding: PhantomData<E>,
}

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq, Hash)]
pub enum StreamingError {
    #[error("The streaming request was interrupted")]
    Interrupted,

    #[error("The stream failed to decode a chunk")]
    Decoding,

    #[error("The streaming request failed")]
    Failed,
}

impl<T, E> Streaming<T, E> {
    /// Creates a new stream from the given stream.
    pub fn new(value: impl Stream<Item = T> + Send + 'static) -> Self {
        // Box and pin the incoming stream and store as a trait object
        Self {
            server_stream: Box::pin(value) as Pin<Box<dyn Stream<Item = T> + Send>>,
            client_stream: None,
            encoding: PhantomData,
        }
    }

    pub async fn next(&mut self) -> Option<Result<T, StreamingError>> {
        self.client_stream.as_mut()?.next().await
    }

    pub fn cancel(self) {}
}

impl<T, E> Streaming<T, E> {
    /// Consumes the wrapper, returning the inner stream.
    pub fn into_inner(self) -> impl Stream<Item = T> + Send {
        self.server_stream
    }
}

impl<T, S, U, E> From<S> for Streaming<T, E>
where
    S: Stream<Item = U> + Send + 'static,
    U: Into<T>,
{
    fn from(value: S) -> Self {
        Self {
            server_stream: Box::pin(value.map(|data| data.into()))
                as Pin<Box<dyn Stream<Item = T> + Send>>,
            client_stream: None,
            encoding: PhantomData,
        }
    }
}

impl IntoResponse for Streaming<String> {
    fn into_response(self) -> axum_core::response::Response {
        axum::response::Response::builder()
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(axum::body::Body::from_stream(
                self.server_stream.map(Result::<String, StreamingError>::Ok),
            ))
            .unwrap()
    }
}
impl IntoResponse for Streaming<Bytes> {
    fn into_response(self) -> axum_core::response::Response {
        axum::response::Response::builder()
            .header("Content-Type", "application/octet-stream")
            .body(axum::body::Body::from_stream(
                self.server_stream.map(Result::<Bytes, StreamingError>::Ok),
            ))
            .unwrap()
    }
}

impl<T: DeserializeOwned + Serialize + 'static, E: Encoding> IntoResponse for Streaming<T, E> {
    fn into_response(self) -> axum_core::response::Response {
        let res = self.server_stream.map(|r| match E::to_bytes(&r) {
            Some(bytes) => Ok(bytes),
            None => Err(StreamingError::Failed),
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
                client_stream: Some(client_stream),
                server_stream: Box::pin(futures::stream::empty())
                    as Pin<Box<dyn Stream<Item = String> + Send>>,
                encoding: PhantomData,
            })
        })
    }
}

impl FromResponse for Streaming<Bytes> {
    fn from_response(res: ClientResponse) -> impl Future<Output = Result<Self, ServerFnError>> {
        async move { todo!() }
    }
}

impl<T: DeserializeOwned + Serialize + 'static + Send, E: Encoding> FromResponse
    for Streaming<T, E>
{
    fn from_response(res: ClientResponse) -> impl Future<Output = Result<Self, ServerFnError>> {
        SendWrapper::new(async move {
            let client_stream = Box::pin(res.bytes_stream().map(|byte| match byte {
                Ok(bytes) => match E::from_bytes(bytes) {
                    Some(res) => Ok(res),
                    None => Err(StreamingError::Decoding),
                },
                Err(_) => Err(StreamingError::Failed),
            }));

            Ok(Self {
                client_stream: Some(client_stream),
                server_stream: Box::pin(futures::stream::empty()),
                encoding: PhantomData,
            })
        })
    }
}

impl<T> std::fmt::Debug for Streaming<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Streaming").finish()
    }
}
