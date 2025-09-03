/// Response types for Actix.
#[cfg(feature = "actix-no-default")]
pub mod actix;
/// Response types for the browser.
#[cfg(feature = "browser")]
pub mod browser;
#[cfg(feature = "generic")]
pub mod generic;
/// Response types for Axum.
#[cfg(feature = "axum-no-default")]
pub mod http;
/// Response types for [`reqwest`].
#[cfg(feature = "reqwest")]
pub mod reqwest;

use bytes::Bytes;
use futures::Stream;
use std::future::Future;

/// Represents the response as created by the server;
pub trait TryRes<E>
where
    Self: Sized,
{
    /// Attempts to convert a UTF-8 string into an HTTP response.
    fn try_from_string(content_type: &str, data: String) -> Result<Self, E>;

    /// Attempts to convert a binary blob represented as bytes into an HTTP response.
    fn try_from_bytes(content_type: &str, data: Bytes) -> Result<Self, E>;

    /// Attempts to convert a stream of bytes into an HTTP response.
    fn try_from_stream(
        content_type: &str,
        data: impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static,
    ) -> Result<Self, E>;
}

/// Represents the response as created by the server;
pub trait Res {
    /// Converts an error into a response, with a `500` status code and the error text as its body.
    fn error_response(path: &str, err: Bytes) -> Self;

    /// Redirect the response by setting a 302 code and Location header.
    fn redirect(&mut self, path: &str);
}

/// Represents the response as received by the client.
pub trait ClientRes<E> {
    /// Attempts to extract a UTF-8 string from an HTTP response.
    fn try_into_string(self) -> impl Future<Output = Result<String, E>> + Send;

    /// Attempts to extract a binary blob from an HTTP response.
    fn try_into_bytes(self) -> impl Future<Output = Result<Bytes, E>> + Send;

    /// Attempts to extract a binary stream from an HTTP response.
    fn try_into_stream(
        self,
    ) -> Result<
        impl Stream<Item = Result<Bytes, Bytes>> + Send + Sync + 'static,
        E,
    >;

    /// HTTP status code of the response.
    fn status(&self) -> u16;

    /// Status text for the status code.
    fn status_text(&self) -> String;

    /// The `Location` header or (if none is set), the URL of the response.
    fn location(&self) -> String;

    /// Whether the response has the [`REDIRECT_HEADER`](crate::redirect::REDIRECT_HEADER) set.
    fn has_redirect(&self) -> bool;
}

/// A mocked response type that can be used in place of the actual server response,
/// when compiling for the browser.
///
/// ## Panics
/// This always panics if its methods are called. It is used solely to stub out the
/// server response type when compiling for the client.
pub struct BrowserMockRes;

impl<E> TryRes<E> for BrowserMockRes {
    fn try_from_string(_content_type: &str, _data: String) -> Result<Self, E> {
        unreachable!()
    }

    fn try_from_bytes(_content_type: &str, _data: Bytes) -> Result<Self, E> {
        unreachable!()
    }

    fn try_from_stream(
        _content_type: &str,
        _data: impl Stream<Item = Result<Bytes, Bytes>>,
    ) -> Result<Self, E> {
        unreachable!()
    }
}

impl Res for BrowserMockRes {
    fn error_response(_path: &str, _err: Bytes) -> Self {
        unreachable!()
    }

    fn redirect(&mut self, _path: &str) {
        unreachable!()
    }
}
