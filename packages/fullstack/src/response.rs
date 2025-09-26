use bytes::Bytes;
use dioxus_fullstack_core::ServerFnError;
use futures::Stream;
use http::HeaderMap;
use reqwest::{RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use std::{future::Future, pin::Pin};
use url::Url;

/// A wrapper type over the platform's HTTP response type.
///
/// This abstracts over the inner `reqwest::Response` type and provides the original request
/// and a way to store state associated with the response.
pub struct ClientResponse {
    pub(crate) inner: reqwest::Response,
    pub(crate) state: Option<Box<dyn std::any::Any + Send + Sync>>,
}
impl ClientResponse {
    pub fn status(&self) -> StatusCode {
        self.inner.status()
    }
    pub fn headers(&self) -> &HeaderMap {
        self.inner.headers()
    }
    pub fn url(&self) -> &Url {
        self.inner.url()
    }
    pub fn content_length(&self) -> Option<u64> {
        self.inner.content_length()
    }
    pub fn bytes(self) -> impl Future<Output = Result<Bytes, reqwest::Error>> {
        self.inner.bytes()
    }
    pub fn bytes_stream(
        self,
    ) -> impl futures_util::Stream<Item = Result<Bytes, reqwest::Error>> + 'static + Unpin {
        self.inner.bytes_stream()
    }
    pub fn original_request(&self) {
        todo!()
    }
    pub fn state<T>(&self) -> &T {
        todo!()
    }
    pub fn json<T: DeserializeOwned>(self) -> impl Future<Output = Result<T, reqwest::Error>> {
        self.inner.json()
    }
    pub fn text(self) -> impl Future<Output = Result<String, reqwest::Error>> {
        self.inner.text()
    }
}

pub struct ClientRequest {}
