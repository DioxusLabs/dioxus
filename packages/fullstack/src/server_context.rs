use dioxus::prelude::ScopeState;

/// A trait for an object that contains a server context
pub trait HasServerContext {
    /// Get the server context from the state
    fn server_context(&self) -> DioxusServerContext;

    /// A shortcut for `self.server_context()`
    fn sc(&self) -> DioxusServerContext {
        self.server_context()
    }
}

impl HasServerContext for &ScopeState {
    fn server_context(&self) -> DioxusServerContext {
        #[cfg(feature = "ssr")]
        {
            self.consume_context().expect("No server context found")
        }
        #[cfg(not(feature = "ssr"))]
        {
            DioxusServerContext {}
        }
    }
}

/// A shared context for server functions that contains infomation about the request and middleware state.
/// This allows you to pass data between your server framework and the server functions. This can be used to pass request information or information about the state of the server. For example, you could pass authentication data though this context to your server functions.
///
/// You should not construct this directly inside components. Instead use the `HasServerContext` trait to get the server context from the scope.
#[derive(Clone)]
pub struct DioxusServerContext {
    #[cfg(feature = "ssr")]
    shared_context: std::sync::Arc<
        std::sync::RwLock<anymap::Map<dyn anymap::any::Any + Send + Sync + 'static>>,
    >,
    #[cfg(feature = "ssr")]
    headers: std::sync::Arc<std::sync::RwLock<hyper::header::HeaderMap>>,
    #[cfg(feature = "ssr")]
    pub(crate) parts: std::sync::Arc<RequestParts>,
}

#[allow(clippy::derivable_impls)]
impl Default for DioxusServerContext {
    fn default() -> Self {
        Self {
            #[cfg(feature = "ssr")]
            shared_context: std::sync::Arc::new(std::sync::RwLock::new(anymap::Map::new())),
            #[cfg(feature = "ssr")]
            headers: Default::default(),
            #[cfg(feature = "ssr")]
            parts: Default::default(),
        }
    }
}

#[cfg(feature = "ssr")]
pub use server_fn_impl::*;

#[cfg(feature = "ssr")]
mod server_fn_impl {
    use super::*;
    use std::sync::LockResult;
    use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

    use anymap::{any::Any, Map};
    type SendSyncAnyMap = Map<dyn Any + Send + Sync + 'static>;

    impl DioxusServerContext {
        /// Create a new server context from a request
        pub fn new(parts: impl Into<Arc<RequestParts>>) -> Self {
            Self {
                parts: parts.into(),
                shared_context: Arc::new(RwLock::new(SendSyncAnyMap::new())),
                headers: Default::default(),
            }
        }

        /// Clone a value from the shared server context
        pub fn get<T: Any + Send + Sync + Clone + 'static>(&self) -> Option<T> {
            self.shared_context.read().ok()?.get::<T>().cloned()
        }

        /// Insert a value into the shared server context
        pub fn insert<T: Any + Send + Sync + 'static>(
            &mut self,
            value: T,
        ) -> Result<(), PoisonError<RwLockWriteGuard<'_, SendSyncAnyMap>>> {
            self.shared_context
                .write()
                .map(|mut map| map.insert(value))
                .map(|_| ())
        }

        /// Get the headers from the server context
        pub fn response_headers(&self) -> RwLockReadGuard<'_, hyper::header::HeaderMap> {
            self.try_response_headers()
                .expect("Failed to get headers from server context")
        }

        /// Try to get the headers from the server context
        pub fn try_response_headers(
            &self,
        ) -> LockResult<RwLockReadGuard<'_, hyper::header::HeaderMap>> {
            self.headers.read()
        }

        /// Get the headers mutably from the server context
        pub fn response_headers_mut(&self) -> RwLockWriteGuard<'_, hyper::header::HeaderMap> {
            self.try_response_headers_mut()
                .expect("Failed to get headers mutably from server context")
        }

        /// Try to get the headers mut from the server context
        pub fn try_response_headers_mut(
            &self,
        ) -> LockResult<RwLockWriteGuard<'_, hyper::header::HeaderMap>> {
            self.headers.write()
        }

        pub(crate) fn take_response_headers(&self) -> hyper::header::HeaderMap {
            let mut headers = self.headers.write().unwrap();
            std::mem::take(&mut *headers)
        }

        /// Get the request that triggered:
        /// - The initial SSR render if called from a ScopeState or ServerFn
        /// - The server function to be called if called from a server function after the initial render
        pub fn request_parts(&self) -> &RequestParts {
            &self.parts
        }
    }

    /// Associated parts of an HTTP Request
    #[derive(Debug, Default)]
    pub struct RequestParts {
        /// The request's method
        pub method: http::Method,
        /// The request's URI
        pub uri: http::Uri,
        /// The request's version
        pub version: http::Version,
        /// The request's headers
        pub headers: http::HeaderMap<http::HeaderValue>,
        /// The request's extensions
        pub extensions: http::Extensions,
    }

    impl From<http::request::Parts> for RequestParts {
        fn from(parts: http::request::Parts) -> Self {
            Self {
                method: parts.method,
                uri: parts.uri,
                version: parts.version,
                headers: parts.headers,
                extensions: parts.extensions,
            }
        }
    }
}
