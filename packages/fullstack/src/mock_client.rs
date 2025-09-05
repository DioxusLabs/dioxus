//! A mock [`crate::client::Client`] implementation used when no client feature is enabled.

use std::future::Future;

use crate::{request::ClientReq, response::ClientRes};
use futures_util::Stream;

/// A placeholder [`crate::client::Client`] used when no client feature is enabled. The
/// [`crate::client::browser::BrowserClient`] is used on web clients, and [`crate::client::reqwest::ReqwestClient`]
/// is used on native clients
#[non_exhaustive]
pub struct MockServerFnClient {}

impl Default for MockServerFnClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MockServerFnClient {
    /// Create a new mock server function client
    pub fn new() -> Self {
        Self {}
    }
}

impl<Error, InputStreamError, OutputStreamError>
    crate::client::Client<Error, InputStreamError, OutputStreamError> for MockServerFnClient
{
    type Request = MockServerFnClientRequest;

    type Response = MockServerFnClientResponse;

    async fn send(_: MockServerFnClientRequest) -> Result<Self::Response, Error> {
        unimplemented!()
    }

    #[allow(unreachable_code)]
    async fn open_websocket(
        _: &str,
    ) -> Result<
        (
            impl Stream<Item = Result<crate::Bytes, crate::Bytes>> + std::marker::Send + 'static,
            impl futures_util::Sink<crate::Bytes> + std::marker::Send + 'static,
        ),
        Error,
    > {
        unimplemented!()
            as Result<
                (
                    futures_util::stream::Once<futures_util::future::Pending<_>>,
                    futures_util::sink::Drain<crate::Bytes>,
                ),
                _,
            >
    }

    fn spawn(_: impl Future<Output = ()> + Send + 'static) {
        unimplemented!()
    }
}

/// A placeholder [`ClientReq`] used when no client feature is enabled.
#[non_exhaustive]
pub struct MockServerFnClientRequest {}

impl<E> ClientReq<E> for MockServerFnClientRequest {
    type FormData = ();

    fn try_new_req_query(_: &str, _: &str, _: &str, _: &str, _: http::Method) -> Result<Self, E> {
        unimplemented!()
    }

    fn try_new_req_text(_: &str, _: &str, _: &str, _: String, _: http::Method) -> Result<Self, E> {
        unimplemented!()
    }

    fn try_new_req_bytes(
        _: &str,
        _: &str,
        _: &str,
        _: bytes::Bytes,
        _: http::Method,
    ) -> Result<Self, E> {
        unimplemented!()
    }

    fn try_new_req_form_data(
        _: &str,
        _: &str,
        _: &str,
        _: Self::FormData,
        _: http::Method,
    ) -> Result<Self, E> {
        unimplemented!()
    }

    fn try_new_req_multipart(
        _: &str,
        _: &str,
        _: Self::FormData,
        _: http::Method,
    ) -> Result<Self, E> {
        unimplemented!()
    }

    fn try_new_req_streaming(
        _: &str,
        _: &str,
        _: &str,
        _: impl Stream<Item = bytes::Bytes> + Send + 'static,
        _: http::Method,
    ) -> Result<Self, E> {
        unimplemented!()
    }
}

/// A placeholder [`ClientRes`] used when no client feature is enabled.
pub struct MockServerFnClientResponse;

impl<E> ClientRes<E> for MockServerFnClientResponse {
    async fn try_into_string(self) -> Result<String, E> {
        unimplemented!()
    }

    async fn try_into_bytes(self) -> Result<bytes::Bytes, E> {
        unimplemented!()
    }

    #[allow(unreachable_code)]
    fn try_into_stream(
        self,
    ) -> Result<impl Stream<Item = Result<bytes::Bytes, bytes::Bytes>> + Send + Sync + 'static, E>
    {
        unimplemented!() as Result<futures_util::stream::Once<futures_util::future::Pending<_>>, _>
    }

    fn status(&self) -> u16 {
        unimplemented!()
    }

    fn status_text(&self) -> String {
        unimplemented!()
    }

    fn location(&self) -> String {
        unimplemented!()
    }

    fn has_redirect(&self) -> bool {
        unimplemented!()
    }
}
