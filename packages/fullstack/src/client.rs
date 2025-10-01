#![allow(unreachable_code)]

use bytes::Bytes;
use dioxus_fullstack_core::RequestError;
use futures::{Stream, TryStreamExt};
use futures_util::stream::StreamExt;
use http::{Extensions, HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use serde::{de::DeserializeOwned, Serialize};
use std::{pin::Pin, prelude::rust_2024::Future, sync::OnceLock};
use url::Url;

use crate::{reqwest_error_to_request_error, StreamingError};

pub static GLOBAL_REQUEST_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

pub type ClientResult = Result<ClientResponse, RequestError>;

pub struct ClientRequest {
    pub url: Url,
    pub headers: HeaderMap,
    pub extensions: Extensions,
    pub method: Method,
}

impl ClientRequest {
    pub fn new(method: http::Method, url: String, params: &impl Serialize) -> Self {
        Self::fetch_inner(method, url, serde_qs::to_string(params).unwrap())
    }

    // Shrink monomorphization bloat by moving this to its own function
    fn fetch_inner(method: http::Method, url: String, query: String) -> ClientRequest {
        #[cfg(not(target_arch = "wasm32"))]
        let (ip, port) = {
            use std::sync::LazyLock;

            static IP: LazyLock<String> =
                LazyLock::new(|| std::env::var("IP").unwrap_or_else(|_| "127.0.0.1".into()));
            static PORT: LazyLock<String> =
                LazyLock::new(|| std::env::var("PORT").unwrap_or_else(|_| "8080".into()));

            (IP.clone(), PORT.clone())
        };

        #[cfg(target_arch = "wasm32")]
        let (ip, port) = ("127.0.0.1", "8080".to_string());

        let url = format!(
            // "http://localhost:{port}{url}{params}",
            "http://{ip}:{port}{url}{params}",
            params = if query.is_empty() {
                "".to_string()
            } else {
                format!("?{}", query)
            }
        )
        .parse()
        .unwrap();

        ClientRequest {
            method,
            url,
            headers: HeaderMap::new(),
            extensions: Extensions::new(),
            // accepts: None,
            // content_type: None,
        }
    }

    pub fn query(self, query: &impl Serialize) -> Self {
        todo!();
        self
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    // pub fn content_type(mut self, content_type: &str) -> Self {
    //     self.content_type = Some(content_type.to_string());
    //     self.header("Content-Type", content_type)
    // }

    // pub fn accepts(mut self, accepts: &str) -> Self {
    //     self.accepts = Some(accepts.to_string());
    //     self.header("Accept", accepts)
    // }

    /// Add a `Header` to this Request.
    pub fn header<K, V>(mut self, key: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        let Ok(value) = value.try_into() else {
            panic!("Failed to convert header value");
            return self;
        };

        let Ok(key): Result<HeaderName, _> = key.try_into() else {
            panic!("Failed to convert header key");
            return self;
        };

        self.headers.append(key, value);
        self
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    /// Creates a new reqwest client with cookies set
    pub fn new_reqwest_client() -> reqwest::Client {
        let mut client = reqwest::Client::builder();

        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::sync::Arc;
            use std::sync::LazyLock;

            static COOKIES: LazyLock<Arc<reqwest::cookie::Jar>> =
                LazyLock::new(|| Arc::new(reqwest::cookie::Jar::default()));

            client = client.cookie_store(true).cookie_provider(COOKIES.clone());
        }

        client.build().unwrap()
    }

    /// Creates a new reqwest request builder with the method, url, and headers set from this ClientRequest
    pub fn new_reqwest_request(&self) -> reqwest::RequestBuilder {
        let client = GLOBAL_REQUEST_CLIENT.get_or_init(|| Self::new_reqwest_client());

        let mut req = client.request(self.method.clone(), self.url.clone());

        for (key, value) in self.headers.iter() {
            req = req.header(key, value);
        }

        req
    }

    #[cfg(feature = "web")]
    pub fn new_gloo_request(&self) -> gloo_net::http::RequestBuilder {
        let mut builder =
            gloo_net::http::RequestBuilder::new(self.url.path()).method(self.method.clone());

        for (key, value) in self.headers.iter() {
            let value = match value.to_str() {
                Ok(v) => v,
                Err(er) => {
                    tracing::error!("Error converting header {key} value: {}", er);
                    continue;
                }
            };

            builder = builder.header(key.as_str(), value);
        }

        builder
    }

    pub async fn send_form(self, data: &impl Serialize) -> Result<ClientResponse, RequestError> {
        // For GET and HEAD requests, we encode the form data as query parameters.
        // For other request methods, we encode the form data as the request body.
        if matches!(*self.method(), Method::GET | Method::HEAD) {
            return self.query(data).send_empty_body().await;
        }

        let body =
            serde_urlencoded::to_string(data).map_err(|err| RequestError::Body(err.to_string()))?;

        self.header("Content-Type", "application/x-www-form-urlencoded")
            .send_body(body)
            .await
    }

    /// Sends the request with an empty body.
    pub async fn send_empty_body(self) -> Result<ClientResponse, RequestError> {
        todo!()
    }

    pub async fn send_bytes(self, bytes: Bytes) -> Self {
        // let client = client.build().unwrap().request(method.clone(), url);
        todo!();
        self
        // Self {
        //     // client: self.client.body(bytes),
        //     method: self.method,
        // }
    }

    pub async fn send_text(
        self,
        text: impl Into<String> + Into<Bytes>,
    ) -> Result<ClientResponse, RequestError> {
        let bytes: Bytes = text.into();
        todo!()
    }

    pub async fn send_body(self, body: impl Into<Bytes>) -> Result<ClientResponse, RequestError> {
        todo!()
    }

    pub async fn send_json(self, json: &impl Serialize) -> Result<ClientResponse, RequestError> {
        self.header("Content-Type", "application/json")
            .send_body(
                serde_json::to_vec(json).map_err(|e| RequestError::Serialization(e.to_string()))?,
            )
            .await
    }

    pub async fn send_body_stream(
        self,
        stream: impl Stream<Item = Result<Bytes, StreamingError>> + Send + 'static,
    ) -> Result<ClientResponse, RequestError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let res = self
                .new_reqwest_request()
                .body(reqwest::Body::wrap_stream(stream))
                .send()
                .await
                .map_err(reqwest_error_to_request_error)?;

            todo!()
        }

        // On the web, we have to buffer the entire stream into a Blob before sending it,
        // since the Fetch API doesn't support streaming request bodies on browsers yet.
        #[cfg(feature = "web")]
        {
            use wasm_bindgen::JsValue;

            // use browser::WrappedGlooResponse;

            // tracing::info!("Sending streaming request to {}", self.url.path());

            // let (res, abort) = browser::streaming_request(
            //     self.url.path(),
            //     self.accepts.as_deref().unwrap_or("application/json"),
            //     self.content_type
            //         .as_deref()
            //         .unwrap_or("application/octet-stream"),
            //     self.method,
            //     stream,
            // )
            // .unwrap();

            // let res = res.send().await.unwrap();

            // let res = WrappedGlooResponse::new(res, abort);

            // return Ok(ClientResponse {
            //     response: Box::new(res),
            // });
            todo!()
        }

        unimplemented!()
    }

    #[cfg(feature = "web")]
    pub async fn send_js_value(
        self,
        value: wasm_bindgen::JsValue,
    ) -> Result<ClientResponse, RequestError> {
        use std::str::FromStr;

        let inner = self
            .new_gloo_request()
            .body(value)
            .unwrap()
            .send()
            .await
            .unwrap();

        let status = inner.status();
        let url = inner.url().parse().unwrap();
        let headers = {
            let mut map = HeaderMap::new();
            for (key, value) in inner.headers().entries() {
                if let Ok(header_value) = http::HeaderValue::from_str(&value) {
                    let header = HeaderName::from_str(&key).unwrap();
                    map.append(header, header_value);
                }
            }
            map
        };

        let content_length = headers
            .get(http::header::CONTENT_LENGTH)
            .and_then(|val| val.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());

        let status = http::StatusCode::from_u16(status).unwrap_or(http::StatusCode::OK);

        Ok(ClientResponse {
            response: Box::new(browser::WrappedGlooResponse {
                inner,
                headers,
                status,
                url,
                content_length,
            }),
        })
    }
}

// On wasm reqwest not being send/sync gets annoying, but it's not relevant since wasm is single-threaded
unsafe impl Send for ClientRequest {}
unsafe impl Sync for ClientRequest {}

/// A wrapper type over the platform's HTTP response type.
///
/// This abstracts over the inner `reqwest::Response` type and provides the original request
/// and a way to store state associated with the response.
///
/// On the web, it uses `web_sys::Response` instead of `reqwest::Response` to avoid pulling in
/// the entire `reqwest` crate and to support native browser APIs.
pub struct ClientResponse {
    pub(crate) response: Box<dyn ClientResponseDriver>,
}

impl ClientResponse {
    pub(crate) fn from_reqwest(response: reqwest::Response) -> Self {
        todo!()
        // ClientResponse { response }
    }

    pub fn status(&self) -> StatusCode {
        self.response.status()
    }

    pub fn headers(&self) -> &HeaderMap {
        self.response.headers()
    }

    pub fn url(&self) -> &Url {
        self.response.url()
    }

    pub fn content_length(&self) -> Option<u64> {
        self.response.content_length()
    }

    pub async fn bytes(self) -> Result<Bytes, RequestError> {
        self.response.bytes().await
        // .map_err(reqwest_error_to_request_error)
    }

    pub fn bytes_stream(
        self,
    ) -> impl futures_util::Stream<Item = Result<Bytes, RequestError>> + 'static + Unpin + Send
    {
        self.response.bytes_stream()
    }

    pub fn original_request(&self) {
        todo!()
    }

    pub fn state<T>(&self) -> &T {
        todo!()
    }

    pub async fn json<T: DeserializeOwned>(self) -> Result<T, RequestError> {
        serde_json::from_slice(&self.bytes().await?)
            .map_err(|e| RequestError::Decode(e.to_string()))
    }

    pub async fn text(self) -> Result<String, RequestError> {
        self.response.text().await
    }

    pub fn make_parts(&self) -> http::response::Parts {
        todo!()
        // let mut response = http::response::Response::builder().status(self.response.status());

        // #[cfg(not(target_arch = "wasm32"))]
        // {
        //     response = response.version(self.response.version());
        // }

        // #[cfg(target_arch = "wasm32")]
        // {
        //     // wasm32 doesn't support HTTP/2 yet, so we'll just set it to HTTP/1.1
        //     response = response.version(http::Version::HTTP_2);
        // }

        // for (key, value) in self.response.headers().iter() {
        //     response = response.header(key, value);
        // }

        // let (parts, _) = response.body(()).unwrap().into_parts();

        // parts
    }

    pub fn into_parts(
        self,
    ) -> (
        http::response::Parts,
        impl Stream<Item = Result<Bytes, RequestError>>,
    ) {
        let parts = self.make_parts();

        (parts, self.bytes_stream())
    }
}

static ROOT_URL: OnceLock<&'static str> = OnceLock::new();

/// Set the root server URL that all server function paths are relative to for the client.
///
/// If this is not set, it defaults to the origin.
pub fn set_server_url(url: &'static str) {
    ROOT_URL.set(url).unwrap();
}

/// Returns the root server URL for all server functions.
pub fn get_server_url() -> &'static str {
    ROOT_URL.get().copied().unwrap_or("")
}

pub trait ClientResponseDriver {
    fn status(&self) -> StatusCode;
    fn headers(&self) -> &HeaderMap;
    fn url(&self) -> &Url;
    fn content_length(&self) -> Option<u64>;
    fn bytes(self: Box<Self>) -> Pin<Box<dyn Future<Output = Result<Bytes, RequestError>> + Send>>;
    fn bytes_stream(
        self: Box<Self>,
    ) -> Pin<Box<dyn Stream<Item = Result<Bytes, RequestError>> + 'static + Unpin + Send>>;
    fn text(self: Box<Self>) -> Pin<Box<dyn Future<Output = Result<String, RequestError>> + Send>>;
}

mod native {
    use crate::{reqwest_error_to_request_error, ClientResponseDriver};
    use bytes::Bytes;
    use dioxus_fullstack_core::RequestError;
    use futures::{TryFutureExt, TryStreamExt};
    use send_wrapper::SendWrapper;
    use std::{pin::Pin, prelude::rust_2024::Future};

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

        fn bytes(
            self: Box<Self>,
        ) -> Pin<Box<dyn Future<Output = Result<Bytes, RequestError>> + Send>> {
            Box::pin(SendWrapper::new(async move {
                reqwest::Response::bytes(*self)
                    .map_err(reqwest_error_to_request_error)
                    .await
            }))
        }

        fn bytes_stream(
            self: Box<Self>,
        ) -> Pin<
            Box<dyn futures::Stream<Item = Result<Bytes, RequestError>> + 'static + Unpin + Send>,
        > {
            Box::pin(SendWrapper::new(
                reqwest::Response::bytes_stream(*self).map_err(reqwest_error_to_request_error),
            ))
        }

        fn text(
            self: Box<Self>,
        ) -> Pin<Box<dyn Future<Output = Result<String, RequestError>> + Send>> {
            Box::pin(SendWrapper::new(async move {
                reqwest::Response::text(*self)
                    .map_err(reqwest_error_to_request_error)
                    .await
            }))
        }
    }
}

#[cfg(feature = "web")]
mod browser {
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

        fn bytes(
            self: Box<Self>,
        ) -> Pin<Box<dyn Future<Output = Result<Bytes, RequestError>> + Send>> {
            Box::pin(SendWrapper::new(async move {
                let bytes = self.inner.binary().await.unwrap();
                Ok(bytes.into())
            }))
        }

        fn bytes_stream(
            self: Box<Self>,
        ) -> Pin<
            Box<dyn futures::Stream<Item = Result<Bytes, RequestError>> + 'static + Unpin + Send>,
        > {
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

        fn text(
            self: Box<Self>,
        ) -> Pin<Box<dyn Future<Output = Result<String, RequestError>> + Send>> {
            Box::pin(SendWrapper::new(async move {
                let text = self.inner.text().await.unwrap();
                Ok(text)
            }))
        }
    }
}
