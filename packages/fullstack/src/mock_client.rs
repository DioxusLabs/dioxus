//! A mock [`server_fn::client::Client`] implementation used when no client feature is enabled.

use std::future::Future;

use futures_util::Stream;
use server_fn::{request::ClientReq, response::ClientRes};

/// A placeholder [`server_fn::client::Client`] used when no client feature is enabled. The
/// [`server_fn::client::browser::BrowserClient`] is used on web clients, and [`server_fn::client::reqwest::ReqwestClient`]
/// is used on native clients
#[non_exhaustive]
pub struct MockServerFnClient {}

impl MockServerFnClient {
    /// Create a new mock server function client
    pub fn new() -> Self {
        Self {}
    }
}

impl<Error, InputStreamError, OutputStreamError>
    server_fn::client::Client<Error, InputStreamError, OutputStreamError> for MockServerFnClient
{
    type Request = MockServerFnClientRequest;

    type Response = MockServerFnClientResponse;

    fn send(_: Self::Request) -> impl Future<Output = Result<Self::Response, Error>> + Send {
        async move { unimplemented!() }
    }

    #[allow(unreachable_code)]
    fn open_websocket(
        _: &str,
    ) -> impl futures_util::Future<
        Output = Result<
            (
                impl Stream<Item = Result<server_fn::Bytes, server_fn::Bytes>>
                    + std::marker::Send
                    + 'static,
                impl futures_util::Sink<server_fn::Bytes> + std::marker::Send + 'static,
            ),
            Error,
        >,
    > + std::marker::Send {
        async move {
            unimplemented!()
                as Result<
                    (
                        futures_util::stream::Once<futures_util::future::Pending<_>>,
                        futures_util::sink::Drain<server_fn::Bytes>,
                    ),
                    _,
                >
        }
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
    fn try_into_string(self) -> impl Future<Output = Result<String, E>> + Send {
        async move { unimplemented!() }
    }

    fn try_into_bytes(self) -> impl Future<Output = Result<bytes::Bytes, E>> + Send {
        async move { unimplemented!() }
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
