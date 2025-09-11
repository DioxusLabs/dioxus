use std::pin::Pin;

use axum::response::IntoResponse;
use futures::{Stream, StreamExt};

use crate::ServerFnError;

/// A stream of text.
///
/// A server function can return this type if its output encoding is [`StreamingText`].
///
/// ## Browser Support for Streaming Input
///
/// Browser fetch requests do not currently support full request duplexing, which
/// means that that they do begin handling responses until the full request has been sent.
/// This means that if you use a streaming input encoding, the input stream needs to
/// end before the output will begin.
///
/// Streaming requests are only allowed over HTTP2 or HTTP3.
pub struct TextStream<E = ServerFnError>(Pin<Box<dyn Stream<Item = Result<String, E>> + Send>>);

impl<E> std::fmt::Debug for TextStream<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("TextStream").finish()
    }
}

impl<E> TextStream<E> {
    /// Creates a new `TextStream` from the given stream.
    pub fn new(value: impl Stream<Item = Result<String, E>> + Send + 'static) -> Self {
        Self(Box::pin(value.map(|value| value)))
    }
}

impl<E> TextStream<E> {
    /// Consumes the wrapper, returning a stream of text.
    pub fn into_inner(self) -> impl Stream<Item = Result<String, E>> + Send {
        self.0
    }
}

impl<E, S, T> From<S> for TextStream<E>
where
    S: Stream<Item = T> + Send + 'static,
    T: Into<String>,
{
    fn from(value: S) -> Self {
        Self(Box::pin(value.map(|data| Ok(data.into()))))
    }
}

impl<E> IntoResponse for TextStream<E> {
    fn into_response(self) -> axum::response::Response {
        todo!()
    }
}
