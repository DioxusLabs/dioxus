// /// Response types for the browser.
// #[cfg(feature = "browser")]
// pub mod browser;

// #[cfg(feature = "generic")]
// pub mod generic;

/// Response types for Axum.
#[cfg(feature = "axum-no-default")]
pub mod http;

/// Response types for [`reqwest`].
#[cfg(feature = "reqwest")]
pub mod reqwest;

use axum::Json;
use bytes::Bytes;
use futures::{FutureExt, Stream};
use std::future::Future;

use crate::{HybridError, HybridResponse};

impl HybridResponse {
    /// Attempts to extract a UTF-8 string from an HTTP response.
    pub async fn try_into_string(self) -> Result<String, HybridError> {
        todo!()
    }

    /// Attempts to extract a binary blob from an HTTP response.
    pub async fn try_into_bytes(self) -> Result<Bytes, HybridError> {
        todo!()
    }

    /// Attempts to extract a binary stream from an HTTP response.
    pub fn try_into_stream(
        self,
    ) -> Result<impl Stream<Item = Result<Bytes, Bytes>> + Send + Sync + 'static, HybridError> {
        Ok(async { todo!() }.into_stream())
    }

    /// HTTP status code of the response.
    pub fn status(&self) -> u16 {
        todo!()
    }

    /// Status text for the status code.
    pub fn status_text(&self) -> String {
        todo!()
    }

    /// The `Location` header or (if none is set), the URL of the response.
    pub fn location(&self) -> String {
        todo!()
    }

    /// Whether the response has the [`REDIRECT_HEADER`](crate::redirect::REDIRECT_HEADER) set.
    pub fn has_redirect(&self) -> bool {
        todo!()
    }
}

pub trait IntoServerFnResponse<Marker> {}

pub struct AxumMarker;
impl<T> IntoServerFnResponse<AxumMarker> for T where T: axum::response::IntoResponse {}

pub struct MyWebSocket {}
pub struct MyWebSocketMarker;
impl IntoServerFnResponse<MyWebSocketMarker> for MyWebSocket {}

// pub struct DefaultEncodingResultMarker;
// impl<T> IntoServerFnResponse<DefaultEncodingResultMarker> for Result<T, HybridError> where
//     T: serde::Serialize
// {
// }

pub struct DefaultEncodingMarker;
impl<T: 'static> IntoServerFnResponse<DefaultEncodingMarker> for Result<T, HybridError> where
    T: serde::Serialize
{
}

fn it_works() {
    // let a = verify(handler_implicit);
    let a = verify(handler_explicit);
    let b = verify(handler_implicit_result);

    // <handler_explicit as IntoServerFnResponse<AxumMarker>>;
}

fn verify<M, F: IntoServerFnResponse<M>>(f: impl Fn() -> F) -> M {
    todo!()
}

#[derive(serde::Serialize, serde::Deserialize)]
struct MyObject {
    id: i32,
    name: String,
}

fn handler_implicit() -> MyObject {
    todo!()
}

fn handler_implicit_result() -> Result<MyObject, HybridError> {
    todo!()
}

fn handler_explicit() -> Json<MyObject> {
    todo!()
}

// pub struct DefaultJsonEncoder<T>(std::marker::PhantomData<T>);

// /// Represents the response as created by the server;
// pub trait Res {
//     /// Converts an error into a response, with a `500` status code and the error text as its body.
//     fn error_response(path: &str, err: Bytes) -> Self;

//     /// Redirect the response by setting a 302 code and Location header.
//     fn redirect(&mut self, path: &str);
// }

// /// Represents the response as received by the client.
// pub trait ClientRes<E> {
//     /// Attempts to extract a UTF-8 string from an HTTP response.
//     fn try_into_string(self) -> impl Future<Output = Result<String, E>> + Send;

//     /// Attempts to extract a binary blob from an HTTP response.
//     fn try_into_bytes(self) -> impl Future<Output = Result<Bytes, E>> + Send;

//     /// Attempts to extract a binary stream from an HTTP response.
//     fn try_into_stream(
//         self,
//     ) -> Result<impl Stream<Item = Result<Bytes, Bytes>> + Send + Sync + 'static, E>;

//     /// HTTP status code of the response.
//     fn status(&self) -> u16;

//     /// Status text for the status code.
//     fn status_text(&self) -> String;

//     /// The `Location` header or (if none is set), the URL of the response.
//     fn location(&self) -> String;

//     /// Whether the response has the [`REDIRECT_HEADER`](crate::redirect::REDIRECT_HEADER) set.
//     fn has_redirect(&self) -> bool;
// }

// /// A mocked response type that can be used in place of the actual server response,
// /// when compiling for the browser.
// ///
// /// ## Panics
// /// This always panics if its methods are called. It is used solely to stub out the
// /// server response type when compiling for the client.
// pub struct BrowserMockRes;

// impl<E> TryRes<E> for BrowserMockRes {
//     fn try_from_string(_content_type: &str, _data: String) -> Result<Self, E> {
//         unreachable!()
//     }

//     fn try_from_bytes(_content_type: &str, _data: Bytes) -> Result<Self, E> {
//         unreachable!()
//     }

//     fn try_from_stream(
//         _content_type: &str,
//         _data: impl Stream<Item = Result<Bytes, Bytes>>,
//     ) -> Result<Self, E> {
//         unreachable!()
//     }
// }

// impl Res for BrowserMockRes {
//     fn error_response(_path: &str, _err: Bytes) -> Self {
//         unreachable!()
//     }

//     fn redirect(&mut self, _path: &str) {
//         unreachable!()
//     }
// }

// /// Represents the response as created by the server;
// pub trait TryRes<E>
// where
//     Self: Sized,
// {
//     /// Attempts to convert a UTF-8 string into an HTTP response.
//     fn try_from_string(content_type: &str, data: String) -> Result<Self, E>;

//     /// Attempts to convert a binary blob represented as bytes into an HTTP response.
//     fn try_from_bytes(content_type: &str, data: Bytes) -> Result<Self, E>;

//     /// Attempts to convert a stream of bytes into an HTTP response.
//     fn try_from_stream(
//         content_type: &str,
//         data: impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static,
//     ) -> Result<Self, E>;
// }
