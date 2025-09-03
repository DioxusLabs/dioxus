//! This module uses platform-agnostic abstractions
//! allowing users to run server functions on a wide range of
//! platforms.
//!
//! The crates in use in this crate are:
//!
//! * `bytes`: platform-agnostic manipulation of bytes.
//! * `http`: low-dependency HTTP abstractions' *front-end*.
//!
//! # Users
//!
//! * `wasm32-wasip*` integration crate `leptos_wasi` is using this
//!   crate under the hood.

use super::{Res, TryRes};
use crate::error::{
    FromServerFnError, IntoAppError, ServerFnErrorErr, ServerFnErrorWrapper,
    SERVER_FN_ERROR_HEADER,
};
use bytes::Bytes;
use futures::{Stream, TryStreamExt};
use http::{header, HeaderValue, Response, StatusCode};
use std::pin::Pin;
use throw_error::Error;

/// The Body of a Response whose *execution model* can be
/// customised using the variants.
pub enum Body {
    /// The response body will be written synchronously.
    Sync(Bytes),

    /// The response body will be written asynchronously,
    /// this execution model is also known as
    /// "streaming".
    Async(Pin<Box<dyn Stream<Item = Result<Bytes, Error>> + Send + 'static>>),
}

impl From<String> for Body {
    fn from(value: String) -> Self {
        Body::Sync(Bytes::from(value))
    }
}

impl From<Bytes> for Body {
    fn from(value: Bytes) -> Self {
        Body::Sync(value)
    }
}

impl<E> TryRes<E> for Response<Body>
where
    E: Send + Sync + FromServerFnError,
{
    fn try_from_string(content_type: &str, data: String) -> Result<Self, E> {
        let builder = http::Response::builder();
        builder
            .status(200)
            .header(http::header::CONTENT_TYPE, content_type)
            .body(data.into())
            .map_err(|e| {
                ServerFnErrorErr::Response(e.to_string()).into_app_error()
            })
    }

    fn try_from_bytes(content_type: &str, data: Bytes) -> Result<Self, E> {
        let builder = http::Response::builder();
        builder
            .status(200)
            .header(http::header::CONTENT_TYPE, content_type)
            .body(Body::Sync(data))
            .map_err(|e| {
                ServerFnErrorErr::Response(e.to_string()).into_app_error()
            })
    }

    fn try_from_stream(
        content_type: &str,
        data: impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static,
    ) -> Result<Self, E> {
        let builder = http::Response::builder();
        builder
            .status(200)
            .header(http::header::CONTENT_TYPE, content_type)
            .body(Body::Async(Box::pin(
                data.map_err(|e| ServerFnErrorWrapper(E::de(e)))
                    .map_err(Error::from),
            )))
            .map_err(|e| {
                ServerFnErrorErr::Response(e.to_string()).into_app_error()
            })
    }
}

impl Res for Response<Body> {
    fn error_response(path: &str, err: Bytes) -> Self {
        Response::builder()
            .status(http::StatusCode::INTERNAL_SERVER_ERROR)
            .header(SERVER_FN_ERROR_HEADER, path)
            .body(err.into())
            .unwrap()
    }

    fn redirect(&mut self, path: &str) {
        if let Ok(path) = HeaderValue::from_str(path) {
            self.headers_mut().insert(header::LOCATION, path);
            *self.status_mut() = StatusCode::FOUND;
        }
    }
}
