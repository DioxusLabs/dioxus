use bytes::Bytes;
use futures::{Sink, Stream};
use http::Method;
use std::{borrow::Cow, future::Future};

/// Request types for Actix.
#[cfg(feature = "actix-no-default")]
pub mod actix;
/// Request types for Axum.
#[cfg(feature = "axum-no-default")]
pub mod axum;
/// Request types for the browser.
#[cfg(feature = "browser")]
pub mod browser;
#[cfg(feature = "generic")]
pub mod generic;
/// Request types for [`reqwest`].
#[cfg(feature = "reqwest")]
pub mod reqwest;

/// Represents a request as made by the client.
pub trait ClientReq<E>
where
    Self: Sized,
{
    /// The type used for URL-encoded form data in this client.
    type FormData;

    /// Attempts to construct a new request with query parameters.
    fn try_new_req_query(
        path: &str,
        content_type: &str,
        accepts: &str,
        query: &str,
        method: Method,
    ) -> Result<Self, E>;

    /// Attempts to construct a new request with a text body.
    fn try_new_req_text(
        path: &str,
        content_type: &str,
        accepts: &str,
        body: String,
        method: Method,
    ) -> Result<Self, E>;

    /// Attempts to construct a new request with a binary body.
    fn try_new_req_bytes(
        path: &str,
        content_type: &str,
        accepts: &str,
        body: Bytes,
        method: Method,
    ) -> Result<Self, E>;

    /// Attempts to construct a new request with form data as the body.
    fn try_new_req_form_data(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: Self::FormData,
        method: Method,
    ) -> Result<Self, E>;

    /// Attempts to construct a new request with a multipart body.
    fn try_new_req_multipart(
        path: &str,
        accepts: &str,
        body: Self::FormData,
        method: Method,
    ) -> Result<Self, E>;

    /// Attempts to construct a new request with a streaming body.
    fn try_new_req_streaming(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: impl Stream<Item = Bytes> + Send + 'static,
        method: Method,
    ) -> Result<Self, E>;

    /// Attempts to construct a new `GET` request.
    fn try_new_get(
        path: &str,
        content_type: &str,
        accepts: &str,
        query: &str,
    ) -> Result<Self, E> {
        Self::try_new_req_query(path, content_type, accepts, query, Method::GET)
    }

    /// Attempts to construct a new `DELETE` request.
    /// **Note**: Browser support for `DELETE` requests without JS/WASM may be poor.
    /// Consider using a `POST` request if functionality without JS/WASM is required.
    fn try_new_delete(
        path: &str,
        content_type: &str,
        accepts: &str,
        query: &str,
    ) -> Result<Self, E> {
        Self::try_new_req_query(
            path,
            content_type,
            accepts,
            query,
            Method::DELETE,
        )
    }

    /// Attempts to construct a new `POST` request with a text body.
    fn try_new_post(
        path: &str,
        content_type: &str,
        accepts: &str,
        body: String,
    ) -> Result<Self, E> {
        Self::try_new_req_text(path, content_type, accepts, body, Method::POST)
    }

    /// Attempts to construct a new `PATCH` request with a text body.
    /// **Note**: Browser support for `PATCH` requests without JS/WASM may be poor.
    /// Consider using a `POST` request if functionality without JS/WASM is required.
    fn try_new_patch(
        path: &str,
        content_type: &str,
        accepts: &str,
        body: String,
    ) -> Result<Self, E> {
        Self::try_new_req_text(path, content_type, accepts, body, Method::PATCH)
    }

    /// Attempts to construct a new `PUT` request with a text body.
    /// **Note**: Browser support for `PUT` requests without JS/WASM may be poor.
    /// Consider using a `POST` request if functionality without JS/WASM is required.
    fn try_new_put(
        path: &str,
        content_type: &str,
        accepts: &str,
        body: String,
    ) -> Result<Self, E> {
        Self::try_new_req_text(path, content_type, accepts, body, Method::PUT)
    }

    /// Attempts to construct a new `POST` request with a binary body.
    fn try_new_post_bytes(
        path: &str,
        content_type: &str,
        accepts: &str,
        body: Bytes,
    ) -> Result<Self, E> {
        Self::try_new_req_bytes(path, content_type, accepts, body, Method::POST)
    }

    /// Attempts to construct a new `PATCH` request with a binary body.
    /// **Note**: Browser support for `PATCH` requests without JS/WASM may be poor.
    /// Consider using a `POST` request if functionality without JS/WASM is required.
    fn try_new_patch_bytes(
        path: &str,
        content_type: &str,
        accepts: &str,
        body: Bytes,
    ) -> Result<Self, E> {
        Self::try_new_req_bytes(
            path,
            content_type,
            accepts,
            body,
            Method::PATCH,
        )
    }

    /// Attempts to construct a new `PUT` request with a binary body.
    /// **Note**: Browser support for `PUT` requests without JS/WASM may be poor.
    /// Consider using a `POST` request if functionality without JS/WASM is required.
    fn try_new_put_bytes(
        path: &str,
        content_type: &str,
        accepts: &str,
        body: Bytes,
    ) -> Result<Self, E> {
        Self::try_new_req_bytes(path, content_type, accepts, body, Method::PUT)
    }

    /// Attempts to construct a new `POST` request with form data as the body.
    fn try_new_post_form_data(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: Self::FormData,
    ) -> Result<Self, E> {
        Self::try_new_req_form_data(
            path,
            accepts,
            content_type,
            body,
            Method::POST,
        )
    }

    /// Attempts to construct a new `PATCH` request with form data as the body.
    /// **Note**: Browser support for `PATCH` requests without JS/WASM may be poor.
    /// Consider using a `POST` request if functionality without JS/WASM is required.
    fn try_new_patch_form_data(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: Self::FormData,
    ) -> Result<Self, E> {
        Self::try_new_req_form_data(
            path,
            accepts,
            content_type,
            body,
            Method::PATCH,
        )
    }

    /// Attempts to construct a new `PUT` request with form data as the body.
    /// **Note**: Browser support for `PUT` requests without JS/WASM may be poor.
    /// Consider using a `POST` request if functionality without JS/WASM is required.
    fn try_new_put_form_data(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: Self::FormData,
    ) -> Result<Self, E> {
        Self::try_new_req_form_data(
            path,
            accepts,
            content_type,
            body,
            Method::PUT,
        )
    }

    /// Attempts to construct a new `POST` request with a multipart body.
    fn try_new_post_multipart(
        path: &str,
        accepts: &str,
        body: Self::FormData,
    ) -> Result<Self, E> {
        Self::try_new_req_multipart(path, accepts, body, Method::POST)
    }

    /// Attempts to construct a new `PATCH` request with a multipart body.
    /// **Note**: Browser support for `PATCH` requests without JS/WASM may be poor.
    /// Consider using a `POST` request if functionality without JS/WASM is required.
    fn try_new_patch_multipart(
        path: &str,
        accepts: &str,
        body: Self::FormData,
    ) -> Result<Self, E> {
        Self::try_new_req_multipart(path, accepts, body, Method::PATCH)
    }

    /// Attempts to construct a new `PUT` request with a multipart body.
    /// **Note**: Browser support for `PUT` requests without JS/WASM may be poor.
    /// Consider using a `POST` request if functionality without JS/WASM is required.
    fn try_new_put_multipart(
        path: &str,
        accepts: &str,
        body: Self::FormData,
    ) -> Result<Self, E> {
        Self::try_new_req_multipart(path, accepts, body, Method::PUT)
    }

    /// Attempts to construct a new `POST` request with a streaming body.
    fn try_new_post_streaming(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: impl Stream<Item = Bytes> + Send + 'static,
    ) -> Result<Self, E> {
        Self::try_new_req_streaming(
            path,
            accepts,
            content_type,
            body,
            Method::POST,
        )
    }

    /// Attempts to construct a new `PATCH` request with a streaming body.
    /// **Note**: Browser support for `PATCH` requests without JS/WASM may be poor.
    /// Consider using a `POST` request if functionality without JS/WASM is required.
    fn try_new_patch_streaming(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: impl Stream<Item = Bytes> + Send + 'static,
    ) -> Result<Self, E> {
        Self::try_new_req_streaming(
            path,
            accepts,
            content_type,
            body,
            Method::PATCH,
        )
    }

    /// Attempts to construct a new `PUT` request with a streaming body.
    /// **Note**: Browser support for `PUT` requests without JS/WASM may be poor.
    /// Consider using a `POST` request if functionality without JS/WASM is required.
    fn try_new_put_streaming(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: impl Stream<Item = Bytes> + Send + 'static,
    ) -> Result<Self, E> {
        Self::try_new_req_streaming(
            path,
            accepts,
            content_type,
            body,
            Method::PUT,
        )
    }
}

/// Represents the request as received by the server.
pub trait Req<Error, InputStreamError = Error, OutputStreamError = Error>
where
    Self: Sized,
{
    /// The response type for websockets.
    type WebsocketResponse: Send;

    /// Returns the query string of the request’s URL, starting after the `?`.
    fn as_query(&self) -> Option<&str>;

    /// Returns the `Content-Type` header, if any.
    fn to_content_type(&self) -> Option<Cow<'_, str>>;

    /// Returns the `Accepts` header, if any.
    fn accepts(&self) -> Option<Cow<'_, str>>;

    /// Returns the `Referer` header, if any.
    fn referer(&self) -> Option<Cow<'_, str>>;

    /// Attempts to extract the body of the request into [`Bytes`].
    fn try_into_bytes(
        self,
    ) -> impl Future<Output = Result<Bytes, Error>> + Send;

    /// Attempts to convert the body of the request into a string.
    fn try_into_string(
        self,
    ) -> impl Future<Output = Result<String, Error>> + Send;

    /// Attempts to convert the body of the request into a stream of bytes.
    fn try_into_stream(
        self,
    ) -> Result<impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static, Error>;

    /// Attempts to convert the body of the request into a websocket handle.
    #[allow(clippy::type_complexity)]
    fn try_into_websocket(
        self,
    ) -> impl Future<
        Output = Result<
            (
                impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static,
                impl Sink<Bytes> + Send + 'static,
                Self::WebsocketResponse,
            ),
            Error,
        >,
    > + Send;
}

/// A mocked request type that can be used in place of the actual server request,
/// when compiling for the browser.
pub struct BrowserMockReq;

impl<Error, InputStreamError, OutputStreamError>
    Req<Error, InputStreamError, OutputStreamError> for BrowserMockReq
where
    Error: Send + 'static,
    InputStreamError: Send + 'static,
    OutputStreamError: Send + 'static,
{
    type WebsocketResponse = crate::response::BrowserMockRes;

    fn as_query(&self) -> Option<&str> {
        unreachable!()
    }

    fn to_content_type(&self) -> Option<Cow<'_, str>> {
        unreachable!()
    }

    fn accepts(&self) -> Option<Cow<'_, str>> {
        unreachable!()
    }

    fn referer(&self) -> Option<Cow<'_, str>> {
        unreachable!()
    }
    async fn try_into_bytes(self) -> Result<Bytes, Error> {
        unreachable!()
    }

    async fn try_into_string(self) -> Result<String, Error> {
        unreachable!()
    }

    fn try_into_stream(
        self,
    ) -> Result<impl Stream<Item = Result<Bytes, Bytes>> + Send, Error> {
        Ok(futures::stream::once(async { unreachable!() }))
    }

    async fn try_into_websocket(
        self,
    ) -> Result<
        (
            impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static,
            impl Sink<Bytes> + Send + 'static,
            Self::WebsocketResponse,
        ),
        Error,
    > {
        #[allow(unreachable_code)]
        Err::<
            (
                futures::stream::Once<std::future::Ready<Result<Bytes, Bytes>>>,
                futures::sink::Drain<Bytes>,
                Self::WebsocketResponse,
            ),
            _,
        >(unreachable!())
    }
}
