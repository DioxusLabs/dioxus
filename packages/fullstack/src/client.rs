#![allow(unreachable_code)]

use crate::{reqwest_error_to_request_error, StreamingError};
use bytes::Bytes;
use dioxus_fullstack_core::RequestError;
use futures::Stream;
use futures::{TryFutureExt, TryStreamExt};
use headers::{ContentType, Header};
use http::{response::Parts, Extensions, HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use send_wrapper::SendWrapper;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::{LazyLock, Mutex, OnceLock};
use std::{fmt::Display, pin::Pin, prelude::rust_2024::Future};
use url::Url;

pub static GLOBAL_REQUEST_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

pub type ClientResult = Result<ClientResponse, RequestError>;

pub struct ClientRequest {
    pub url: Url,
    pub headers: HeaderMap,
    pub method: Method,
    pub extensions: Extensions,
}

impl ClientRequest {
    /// Create a new ClientRequest with the given method, url path, and query parameters.
    pub fn new(method: http::Method, path: String, params: &impl Serialize) -> Self {
        Self::fetch_inner(method, path, serde_qs::to_string(params).unwrap())
    }

    // Shrink monomorphization bloat by moving this to its own function
    fn fetch_inner(method: http::Method, path: String, query: String) -> ClientRequest {
        // On wasm, this doesn't matter since we always use relative URLs when making requests anyways
        let mut server_url = get_server_url();

        if server_url.is_empty() {
            server_url = "http://this.is.not.a.real.url:9000";
        }

        let url = format!(
            "{server_url}{path}{params}",
            params = if query.is_empty() {
                "".to_string()
            } else {
                format!("?{}", query)
            }
        )
        .parse()
        .unwrap();

        let headers = get_request_headers();

        ClientRequest {
            method,
            url,
            headers,
            extensions: Extensions::new(),
        }
    }

    /// Get the HTTP method of this Request.
    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    /// Extend the query parameters of this request with the given serialzable struct.
    ///
    /// This will use `serde_qs` to serialize the struct into query parameters. `serde_qs` has various
    /// restrictions - make sure to read its documentation!
    pub fn extend_query(mut self, query: &impl Serialize) -> Self {
        let old_query = self.url.query().unwrap_or("");
        let new_query = serde_qs::to_string(query).unwrap();
        let combined_query = format!(
            "{}{}{}",
            old_query,
            if old_query.is_empty() { "" } else { "&" },
            new_query
        );
        self.url.set_query(Some(&combined_query));
        self
    }

    /// Add a `Header` to this Request.
    pub fn header(
        mut self,
        name: impl TryInto<HeaderName, Error = impl Display>,
        value: impl TryInto<HeaderValue, Error = impl Display>,
    ) -> Result<Self, RequestError> {
        self.headers.append(
            name.try_into()
                .map_err(|d| RequestError::Builder(d.to_string()))?,
            value
                .try_into()
                .map_err(|d| RequestError::Builder(d.to_string()))?,
        );
        Ok(self)
    }

    /// Add a `Header` to this Request.
    pub fn typed_header<H: Header>(mut self, header: H) -> Self {
        let mut headers = vec![];
        header.encode(&mut headers);
        for header in headers {
            self.headers.append(H::name(), header);
        }
        self
    }

    /// Creates a new reqwest client with cookies set
    pub fn new_reqwest_client() -> reqwest::Client {
        #[allow(unused_mut)]
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
    ///
    /// Using this method attaches `X-Request-Client: dioxus` header to the request.
    pub fn new_reqwest_request(&self) -> reqwest::RequestBuilder {
        let client = GLOBAL_REQUEST_CLIENT.get_or_init(Self::new_reqwest_client);

        let mut req = client
            .request(self.method.clone(), self.url.clone())
            .header("X-Request-Client", "dioxus");

        for (key, value) in self.headers.iter() {
            req = req.header(key, value);
        }

        req
    }

    /// Using this method attaches `X-Request-Client-Dioxus` header to the request.
    #[cfg(feature = "web")]
    pub fn new_gloo_request(&self) -> gloo_net::http::RequestBuilder {
        let mut builder = gloo_net::http::RequestBuilder::new(
            format!(
                "{path}{query_string}",
                path = self.url.path(),
                query_string = self
                    .url
                    .query()
                    .map(|query| format!("?{query}"))
                    .unwrap_or_default()
            )
            .as_str(),
        )
        .header("X-Request-Client", "dioxus")
        .method(self.method.clone());

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

    /// Sends the request with multipart/form-data body constructed from the given FormData.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn send_multipart(
        self,
        form: &dioxus_html::FormData,
    ) -> Result<ClientResponse, RequestError> {
        let mut outgoing = reqwest::multipart::Form::new();

        for (key, value) in form.values() {
            match value {
                dioxus_html::FormValue::Text(text) => {
                    outgoing = outgoing.text(key.to_string(), text.to_string());
                }
                dioxus_html::FormValue::File(Some(file_data)) => {
                    outgoing = outgoing
                        .file(key.to_string(), file_data.path())
                        .await
                        .map_err(|e| {
                            RequestError::Builder(format!(
                                "Failed to add file to multipart form: {e}",
                            ))
                        })?;
                }
                dioxus_html::FormValue::File(None) => {
                    // No file was selected for this input, so we skip it.
                    outgoing = outgoing.part(key.to_string(), reqwest::multipart::Part::bytes(b""));
                }
            }
        }

        let res = self
            .new_reqwest_request()
            .multipart(outgoing)
            .send()
            .await
            .map_err(reqwest_error_to_request_error)?;

        Ok(ClientResponse {
            response: Box::new(res),
            extensions: self.extensions,
        })
    }

    pub async fn send_form(self, data: &impl Serialize) -> Result<ClientResponse, RequestError> {
        // For GET and HEAD requests, we encode the form data as query parameters.
        // For other request methods, we encode the form data as the request body.
        if matches!(*self.method(), Method::GET | Method::HEAD) {
            return self.extend_query(data).send_empty_body().await;
        }

        let body =
            serde_urlencoded::to_string(data).map_err(|err| RequestError::Body(err.to_string()))?;

        self.typed_header(ContentType::form_url_encoded())
            .send_raw_bytes(body)
            .await
    }

    /// Sends the request with an empty body.
    pub async fn send_empty_body(self) -> Result<ClientResponse, RequestError> {
        #[cfg(feature = "web")]
        if cfg!(target_arch = "wasm32") {
            return self.send_js_value(wasm_bindgen::JsValue::UNDEFINED).await;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let res = self
                .new_reqwest_request()
                .send()
                .await
                .map_err(reqwest_error_to_request_error)?;

            return Ok(ClientResponse {
                response: Box::new(res),
                extensions: self.extensions,
            });
        }

        unimplemented!()
    }

    pub async fn send_raw_bytes(
        self,
        bytes: impl Into<Bytes>,
    ) -> Result<ClientResponse, RequestError> {
        #[cfg(feature = "web")]
        if cfg!(target_arch = "wasm32") {
            let bytes = bytes.into();
            let uint_8_array = js_sys::Uint8Array::from(&bytes[..]);
            return self.send_js_value(uint_8_array.into()).await;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let res = self
                .new_reqwest_request()
                .body(bytes.into())
                .send()
                .await
                .map_err(reqwest_error_to_request_error)?;

            return Ok(ClientResponse {
                response: Box::new(res),
                extensions: self.extensions,
            });
        }

        unimplemented!()
    }

    /// Sends text data with the `text/plain; charset=utf-8` content type.
    pub async fn send_text(
        self,
        text: impl Into<String> + Into<Bytes>,
    ) -> Result<ClientResponse, RequestError> {
        self.typed_header(ContentType::text_utf8())
            .send_raw_bytes(text)
            .await
    }

    /// Sends JSON data with the `application/json` content type.
    pub async fn send_json(self, json: &impl Serialize) -> Result<ClientResponse, RequestError> {
        let bytes =
            serde_json::to_vec(json).map_err(|e| RequestError::Serialization(e.to_string()))?;

        if bytes.is_empty() || bytes == b"{}" || bytes == b"null" {
            return self.send_empty_body().await;
        }

        self.typed_header(ContentType::json())
            .send_raw_bytes(bytes)
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

            return Ok(ClientResponse {
                response: Box::new(res),
                extensions: self.extensions,
            });
        }

        // On the web, we have to buffer the entire stream into a Blob before sending it,
        // since the Fetch API doesn't support streaming request bodies on browsers yet.
        #[cfg(feature = "web")]
        {
            use wasm_bindgen::JsValue;

            let stream: Vec<Bytes> = stream.try_collect().await.map_err(|e| {
                RequestError::Request(format!("Error collecting stream for request body: {}", e))
            })?;

            let uint_8_array =
                js_sys::Uint8Array::new_with_length(stream.iter().map(|b| b.len() as u32).sum());

            let mut offset = 0;
            for chunk in stream {
                uint_8_array.set(&js_sys::Uint8Array::from(&chunk[..]), offset);
                offset += chunk.len() as u32;
            }

            return self.send_js_value(JsValue::from(uint_8_array)).await;
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
            .map_err(|e| RequestError::Request(e.to_string()))?
            .send()
            .await
            .map_err(|e| RequestError::Request(e.to_string()))?;

        let status = inner.status();
        let url = inner
            .url()
            .parse()
            .map_err(|e| RequestError::Request(format!("Error parsing response URL: {}", e)))?;

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
            extensions: self.extensions,
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
    pub(crate) extensions: Extensions,
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
        self.response.bytes().await
    }

    pub fn bytes_stream(
        self,
    ) -> impl futures_util::Stream<Item = Result<Bytes, StreamingError>> + 'static + Unpin + Send
    {
        self.response.bytes_stream()
    }

    pub fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }

    pub async fn json<T: DeserializeOwned>(self) -> Result<T, RequestError> {
        serde_json::from_slice(&self.bytes().await?)
            .map_err(|e| RequestError::Decode(e.to_string()))
    }

    pub async fn text(self) -> Result<String, RequestError> {
        self.response.text().await
    }

    /// Creates the `http::response::Parts` from this response.
    pub fn make_parts(&self) -> Parts {
        let mut response = http::response::Response::builder().status(self.response.status());

        response = response.version(self.response.version());

        for (key, value) in self.response.headers().iter() {
            response = response.header(key, value);
        }

        let (parts, _) = response.body(()).unwrap().into_parts();

        parts
    }

    /// Consumes the response, returning the head and a stream of the body.
    pub fn into_parts(self) -> (Parts, impl Stream<Item = Result<Bytes, StreamingError>>) {
        (self.make_parts(), self.bytes_stream())
    }
}

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

static ROOT_URL: OnceLock<&'static str> = OnceLock::new();

/// Delete the extra request headers for all server functions.
pub fn clear_request_headers() {
    REQUEST_HEADERS.lock().unwrap().clear();
}

/// Set the extra request headers for all server functions.
pub fn set_request_headers(headers: HeaderMap) {
    *REQUEST_HEADERS.lock().unwrap() = headers;
}

/// Returns the extra request headers for all server functions.
pub fn get_request_headers() -> HeaderMap {
    REQUEST_HEADERS.lock().unwrap().clone()
}

static REQUEST_HEADERS: LazyLock<Mutex<HeaderMap>> = LazyLock::new(|| Mutex::new(HeaderMap::new()));

pub trait ClientResponseDriver {
    fn status(&self) -> StatusCode;
    fn headers(&self) -> &HeaderMap;
    fn url(&self) -> &Url;
    fn version(&self) -> http::Version {
        http::Version::HTTP_2
    }
    fn content_length(&self) -> Option<u64>;
    fn bytes(self: Box<Self>) -> Pin<Box<dyn Future<Output = Result<Bytes, RequestError>> + Send>>;
    fn bytes_stream(
        self: Box<Self>,
    ) -> Pin<Box<dyn Stream<Item = Result<Bytes, StreamingError>> + 'static + Unpin + Send>>;

    fn text(self: Box<Self>) -> Pin<Box<dyn Future<Output = Result<String, RequestError>> + Send>>;
}

mod native {
    use futures::Stream;

    use super::*;

    impl ClientResponseDriver for reqwest::Response {
        fn status(&self) -> http::StatusCode {
            reqwest::Response::status(self)
        }

        fn version(&self) -> http::Version {
            #[cfg(target_arch = "wasm32")]
            {
                return http::Version::HTTP_2;
            }

            reqwest::Response::version(self)
        }

        fn headers(&self) -> &http::HeaderMap {
            reqwest::Response::headers(self)
        }

        fn url(&self) -> &url::Url {
            reqwest::Response::url(self)
        }

        fn content_length(&self) -> Option<u64> {
            reqwest::Response::content_length(self)
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
        ) -> Pin<Box<dyn Stream<Item = Result<Bytes, StreamingError>> + 'static + Unpin + Send>>
        {
            Box::pin(SendWrapper::new(
                reqwest::Response::bytes_stream(*self).map_err(|_| StreamingError::Failed),
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
    use crate::{ClientResponseDriver, StreamingError};
    use bytes::Bytes;
    use dioxus_fullstack_core::RequestError;
    use futures::{Stream, StreamExt};
    use http::{HeaderMap, StatusCode};
    use js_sys::Uint8Array;
    use send_wrapper::SendWrapper;
    use std::{pin::Pin, prelude::rust_2024::Future};
    use wasm_bindgen::JsCast;

    pub(crate) struct WrappedGlooResponse {
        pub(crate) inner: gloo_net::http::Response,
        pub(crate) headers: HeaderMap,
        pub(crate) status: StatusCode,
        pub(crate) url: url::Url,
        pub(crate) content_length: Option<u64>,
    }

    impl ClientResponseDriver for WrappedGlooResponse {
        fn status(&self) -> StatusCode {
            self.status
        }

        fn headers(&self) -> &HeaderMap {
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
                let bytes = self
                    .inner
                    .binary()
                    .await
                    .map_err(|e| RequestError::Request(e.to_string()))?;
                Ok(bytes.into())
            }))
        }

        fn bytes_stream(
            self: Box<Self>,
        ) -> Pin<Box<dyn Stream<Item = Result<Bytes, StreamingError>> + 'static + Unpin + Send>>
        {
            let body = match self.inner.body() {
                Some(body) => body,
                None => {
                    return Box::pin(SendWrapper::new(futures::stream::iter([Err(
                        StreamingError::Failed,
                    )])));
                }
            };

            Box::pin(SendWrapper::new(
                wasm_streams::ReadableStream::from_raw(body)
                    .into_stream()
                    .map(|chunk| {
                        let array = chunk
                            .map_err(|_| StreamingError::Failed)?
                            .dyn_into::<Uint8Array>()
                            .map_err(|_| StreamingError::Failed)?;
                        Ok(array.to_vec().into())
                    }),
            ))
        }

        fn text(
            self: Box<Self>,
        ) -> Pin<Box<dyn Future<Output = Result<String, RequestError>> + Send>> {
            Box::pin(SendWrapper::new(async move {
                self.inner
                    .text()
                    .await
                    .map_err(|e| RequestError::Request(e.to_string()))
            }))
        }
    }
}
