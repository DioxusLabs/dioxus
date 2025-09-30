use bytes::Bytes;
use dioxus_fullstack_core::RequestError;
use futures::{Stream, StreamExt};
use http::{HeaderMap, HeaderName, Method};
use js_sys::Uint8Array;
use send_wrapper::SendWrapper;
use std::{pin::Pin, prelude::rust_2024::Future, str::FromStr};
use wasm_bindgen::{JsCast, JsValue};

use crate::{ClientResponse, ClientResponseDriver};

pub struct WrappedGlooResponse {
    pub(crate) inner: gloo_net::http::Response,
    pub(crate) headers: HeaderMap,
    pub(crate) status: http::StatusCode,
    pub(crate) url: url::Url,
    pub(crate) content_length: Option<u64>,
}

impl ClientResponseDriver for WrappedGlooResponse {
    fn status(&self) -> http::StatusCode {
        self.status
    }

    fn headers(&self) -> &http::HeaderMap {
        &self.headers
    }

    fn url(&self) -> &url::Url {
        &self.url
    }

    fn content_length(&self) -> Option<u64> {
        self.content_length
    }

    fn bytes(self: Box<Self>) -> Pin<Box<dyn Future<Output = Result<Bytes, RequestError>> + Send>> {
        Box::pin(SendWrapper::new(async move {
            let bytes = self.inner.binary().await.unwrap();
            Ok(bytes.into())
        }))
    }

    fn bytes_stream(
        self: Box<Self>,
    ) -> Pin<Box<dyn futures::Stream<Item = Result<Bytes, RequestError>> + 'static + Unpin + Send>>
    {
        let stream = wasm_streams::ReadableStream::from_raw(self.inner.body().unwrap());
        Box::pin(SendWrapper::new(stream.into_stream().map(|chunk| {
            Ok(chunk
                .unwrap()
                .dyn_into::<Uint8Array>()
                .unwrap()
                .to_vec()
                .into())
        })))
    }

    fn text(self: Box<Self>) -> Pin<Box<dyn Future<Output = Result<String, RequestError>> + Send>> {
        Box::pin(SendWrapper::new(async move {
            let text = self.inner.text().await.unwrap();
            Ok(text)
        }))
    }
}
