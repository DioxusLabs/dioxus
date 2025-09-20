use std::{pin::Pin, prelude::rust_2024::Future};

use axum_core::response::IntoResponse;
use futures::{Stream, StreamExt};

use crate::{FromResponse, ServerFnError};

pub type TextStream = Streaming<String, ServerFnError>;

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
pub struct Streaming<T = String, E = ServerFnError> {
    stream: Pin<Box<dyn Stream<Item = Result<T, E>> + Send>>,
}

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq, Hash)]
pub enum StreamingError {
    #[error("The streaming request was interrupted")]
    Interrupted,

    #[error("The streaming request failed")]
    Failed,
}

impl<T, E> std::fmt::Debug for Streaming<T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Streaming").finish()
    }
}

impl<T, E> Streaming<T, E> {
    /// Creates a new stream from the given stream.
    pub fn new(value: impl Stream<Item = Result<T, E>> + Send + 'static) -> Self {
        // Box and pin the incoming stream and store as a trait object
        Self {
            stream: Box::pin(value) as Pin<Box<dyn Stream<Item = Result<T, E>> + Send>>,
        }
    }

    pub async fn next(&mut self) -> Option<Result<T, StreamingError>> {
        todo!()
    }
}

impl<T, E> Streaming<T, E> {
    /// Consumes the wrapper, returning the inner stream.
    pub fn into_inner(self) -> impl Stream<Item = Result<T, E>> + Send {
        self.stream
    }
}

impl<T, E, S, U> From<S> for Streaming<T, E>
where
    S: Stream<Item = U> + Send + 'static,
    U: Into<T>,
{
    fn from(value: S) -> Self {
        Self {
            stream: Box::pin(value.map(|data| Ok(data.into())))
                as Pin<Box<dyn Stream<Item = Result<T, E>> + Send>>,
        }
    }
}

impl<T, E> IntoResponse for Streaming<T, E> {
    fn into_response(self) -> axum_core::response::Response {
        todo!()
    }
}

impl<T, E> FromResponse for Streaming<T, E> {
    fn from_response(
        res: reqwest::Response,
    ) -> impl Future<Output = Result<Self, ServerFnError>> + Send {
        async move { todo!() }
    }
}
