use std::{pin::Pin, prelude::rust_2024::Future, str::FromStr};

use bytes::Bytes;
use dioxus_fullstack_core::RequestError;
use futures::{FutureExt, Stream, StreamExt, TryFutureExt, TryStreamExt};
use http::{HeaderMap, HeaderName, Method};
use js_sys::Uint8Array;
use send_wrapper::SendWrapper;

use crate::{reqwest_error_to_request_error, ClientResponseDriver};

impl ClientResponseDriver for reqwest::Response {
    fn status(&self) -> http::StatusCode {
        self.status()
    }

    fn headers(&self) -> &http::HeaderMap {
        self.headers()
    }

    fn url(&self) -> &url::Url {
        self.url()
    }

    fn content_length(&self) -> Option<u64> {
        self.content_length()
    }

    fn bytes(self: Box<Self>) -> Pin<Box<dyn Future<Output = Result<Bytes, RequestError>> + Send>> {
        Box::pin(SendWrapper::new(async move {
            reqwest::Response::bytes(*self)
                .map_err(reqwest_error_to_request_error)
                .await
        }))
    }

    fn bytes_stream(
        self: Box<Self>,
    ) -> Pin<Box<dyn futures::Stream<Item = Result<Bytes, RequestError>> + 'static + Unpin + Send>>
    {
        Box::pin(SendWrapper::new(
            reqwest::Response::bytes_stream(*self).map_err(reqwest_error_to_request_error),
        ))
    }

    fn text(self: Box<Self>) -> Pin<Box<dyn Future<Output = Result<String, RequestError>> + Send>> {
        Box::pin(SendWrapper::new(async move {
            reqwest::Response::text(*self)
                .map_err(reqwest_error_to_request_error)
                .await
        }))
    }
}
