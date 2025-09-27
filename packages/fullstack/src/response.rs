use bytes::Bytes;
use dioxus_fullstack_core::{RequestError, ServerFnError};
use futures::{Stream, TryStreamExt};
use http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use serde::{de::DeserializeOwned, Serialize};
use std::{future::Future, pin::Pin};
use url::Url;

use crate::{reqwest_error_to_request_error, reqwest_response_to_serverfn_err};

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
    pub async fn bytes(self) -> Result<Bytes, RequestError> {
        self.inner
            .bytes()
            .await
            .map_err(reqwest_error_to_request_error)
    }
    pub fn bytes_stream(
        self,
    ) -> impl futures_util::Stream<Item = Result<Bytes, RequestError>> + 'static + Unpin {
        self.inner
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
        self.inner
            .json()
            .await
            .map_err(reqwest_error_to_request_error)
    }
    pub async fn text(self) -> Result<String, RequestError> {
        self.inner
            .text()
            .await
            .map_err(reqwest_error_to_request_error)
    }
    pub fn make_parts(&self) -> http::response::Parts {
        let mut response = http::response::Response::builder().status(self.inner.status());

        #[cfg(not(target_arch = "wasm32"))]
        {
            response = response.version(self.inner.version());
        }

        #[cfg(target_arch = "wasm32")]
        {
            // wasm32 doesn't support HTTP/2 yet, so we'll just set it to HTTP/1.1
            response = response.version(http::Version::HTTP_2);
        }

        for (key, value) in self.inner.headers().iter() {
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

        Ok(ClientResponse {
            inner: res,
            state: None,
        })
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
}

// pub use reqwest::RequestBuilder;

// /// A wrapper type over the platform's HTTP request type.
// pub struct RequestBuilder {}

// impl RequestBuilder {
//     /// Constructs a new request.
//     #[inline]
//     pub fn new(method: Method, url: Url) -> Self {
//         Request {
//             method,
//             url,
//             headers: HeaderMap::new(),
//             body: None,
//             version: Version::default(),
//             extensions: Extensions::new(),
//         }
//     }

//     /// Get the method.
//     #[inline]
//     pub fn method(&self) -> &Method {
//         &self.method
//     }

//     /// Get a mutable reference to the method.
//     #[inline]
//     pub fn method_mut(&mut self) -> &mut Method {
//         &mut self.method
//     }

//     /// Get the url.
//     #[inline]
//     pub fn url(&self) -> &Url {
//         &self.url
//     }

//     /// Get a mutable reference to the url.
//     #[inline]
//     pub fn url_mut(&mut self) -> &mut Url {
//         &mut self.url
//     }

//     /// Get the headers.
//     #[inline]
//     pub fn headers(&self) -> &HeaderMap {
//         &self.headers
//     }

//     /// Get a mutable reference to the headers.
//     #[inline]
//     pub fn headers_mut(&mut self) -> &mut HeaderMap {
//         &mut self.headers
//     }

//     /// Get the body.
//     #[inline]
//     pub fn body(&self) -> Option<&Body> {
//         self.body.as_ref()
//     }

//     /// Get a mutable reference to the body.
//     #[inline]
//     pub fn body_mut(&mut self) -> &mut Option<Body> {
//         &mut self.body
//     }

//     /// Get the extensions.
//     #[inline]
//     pub(crate) fn extensions(&self) -> &Extensions {
//         &self.extensions
//     }

//     /// Get a mutable reference to the extensions.
//     #[inline]
//     pub(crate) fn extensions_mut(&mut self) -> &mut Extensions {
//         &mut self.extensions
//     }

//     /// Get the timeout.
//     #[inline]
//     pub fn timeout(&self) -> Option<&Duration> {
//         RequestConfig::<RequestTimeout>::get(&self.extensions)
//     }

//     /// Get a mutable reference to the timeout.
//     #[inline]
//     pub fn timeout_mut(&mut self) -> &mut Option<Duration> {
//         RequestConfig::<RequestTimeout>::get_mut(&mut self.extensions)
//     }

//     /// Get the http version.
//     #[inline]
//     pub fn version(&self) -> Version {
//         self.version
//     }

//     /// Get a mutable reference to the http version.
//     #[inline]
//     pub fn version_mut(&mut self) -> &mut Version {
//         &mut self.version
//     }

//     /// Attempt to clone the request.
//     ///
//     /// `None` is returned if the request can not be cloned, i.e. if the body is a stream.
//     pub fn try_clone(&self) -> Option<Request> {
//         let body = match self.body.as_ref() {
//             Some(body) => Some(body.try_clone()?),
//             None => None,
//         };
//         let mut req = Request::new(self.method().clone(), self.url().clone());
//         *req.timeout_mut() = self.timeout().copied();
//         *req.headers_mut() = self.headers().clone();
//         *req.version_mut() = self.version();
//         *req.extensions_mut() = self.extensions().clone();
//         req.body = body;
//         Some(req)
//     }
// }
