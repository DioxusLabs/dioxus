use bytes::Bytes;
use dioxus_fullstack_core::RequestError;
use futures::{Stream, TryStreamExt};
use http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use serde::{de::DeserializeOwned, Serialize};
use url::Url;

use crate::reqwest_error_to_request_error;

pub type ClientResult = Result<ClientResponse, RequestError>;

/// A wrapper type over the platform's HTTP response type.
///
/// This abstracts over the inner `reqwest::Response` type and provides the original request
/// and a way to store state associated with the response.
pub struct ClientResponse {
    pub(crate) response: reqwest::Response,
}

impl ClientResponse {
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
        self.response
            .bytes()
            .await
            .map_err(reqwest_error_to_request_error)
    }
    pub fn bytes_stream(
        self,
    ) -> impl futures_util::Stream<Item = Result<Bytes, RequestError>> + 'static + Unpin {
        self.response
            .bytes_stream()
            .map_err(|e| reqwest_error_to_request_error(e))
    }
    pub fn original_request(&self) {
        todo!()
    }
    pub fn state<T>(&self) -> &T {
        todo!()
    }
    pub async fn json<T: DeserializeOwned>(self) -> Result<T, RequestError> {
        self.response
            .json()
            .await
            .map_err(reqwest_error_to_request_error)
    }
    pub async fn text(self) -> Result<String, RequestError> {
        self.response
            .text()
            .await
            .map_err(reqwest_error_to_request_error)
    }
    pub fn make_parts(&self) -> http::response::Parts {
        let mut response = http::response::Response::builder().status(self.response.status());

        #[cfg(not(target_arch = "wasm32"))]
        {
            response = response.version(self.response.version());
        }

        #[cfg(target_arch = "wasm32")]
        {
            // wasm32 doesn't support HTTP/2 yet, so we'll just set it to HTTP/1.1
            response = response.version(http::Version::HTTP_2);
        }

        for (key, value) in self.response.headers().iter() {
            response = response.header(key, value);
        }

        let (parts, _) = response.body(()).unwrap().into_parts();

        parts
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

pub struct ClientRequest {
    pub client: reqwest::RequestBuilder,
    pub method: Method,
}

// On wasm reqwest not being send/sync gets annoying, but it's not relevant since wasm is single-threaded
unsafe impl Send for ClientRequest {}
unsafe impl Sync for ClientRequest {}

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
        );

        let mut client = reqwest::Client::builder();

        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::sync::Arc;
            use std::sync::LazyLock;

            static COOKIES: LazyLock<Arc<reqwest::cookie::Jar>> =
                LazyLock::new(|| Arc::new(reqwest::cookie::Jar::default()));

            client = client.cookie_store(true).cookie_provider(COOKIES.clone());
        }

        let client = client.build().unwrap().request(method.clone(), url);

        ClientRequest { client, method }
    }

    pub fn query(self, query: &impl Serialize) -> Self {
        Self {
            client: self.client.query(query),
            method: self.method,
        }
    }

    pub fn bytes(self, bytes: Bytes) -> Self {
        Self {
            client: self.client.body(bytes),
            method: self.method,
        }
    }

    pub fn text(self, text: impl Into<String> + Into<Bytes>) -> Self {
        let bytes: Bytes = text.into();
        Self {
            client: self.client.body(bytes),
            method: self.method,
        }
    }

    pub fn body(self, body: impl Into<Bytes>) -> Self {
        Self {
            client: self.client.body(body.into()),
            method: self.method,
        }
    }

    pub fn json(self, json: &impl Serialize) -> Self {
        Self {
            client: self.client.json(json),
            method: self.method,
        }
    }

    pub async fn send(self) -> Result<ClientResponse, RequestError> {
        let res = self
            .client
            .send()
            .await
            .map_err(reqwest_error_to_request_error)?;

        Ok(ClientResponse { response: todo!() })
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    /// Add a `Header` to this Request.
    pub fn header<K, V>(mut self, key: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        self.client = self.client.header(key, value);
        self
    }

    // https://stackoverflow.com/questions/39280438/fetch-missing-boundary-in-multipart-form-data-post
    #[cfg(feature = "web")]
    pub async fn send_form(self, data: web_sys::FormData) {
        todo!()
        // let form = reqwest::multipart::Form::new();
        // for entry in data.entries() {}
    }
}
