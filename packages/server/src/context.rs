use enumset::{EnumSet, EnumSetType};
use parking_lot::RwLock;
use std::any::Any;
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;

type SendSyncAnyMap = std::collections::HashMap<std::any::TypeId, ContextType>;

#[derive(EnumSetType)]
enum ResponsePartsModified {
    Version,
    Headers,
    Status,
    Extensions,
    Body,
}

struct AtomicResponsePartsModified {
    modified: AtomicU32,
}

impl AtomicResponsePartsModified {
    fn new() -> Self {
        Self {
            modified: AtomicU32::new(EnumSet::<ResponsePartsModified>::empty().as_u32()),
        }
    }

    fn set(&self, part: ResponsePartsModified) {
        let modified =
            EnumSet::from_u32(self.modified.load(std::sync::atomic::Ordering::Relaxed)) | part;
        self.modified
            .store(modified.as_u32(), std::sync::atomic::Ordering::Relaxed);
    }

    fn is_modified(&self, part: ResponsePartsModified) -> bool {
        self.modified.load(std::sync::atomic::Ordering::Relaxed) & (1 << part as usize) != 0
    }
}

/// A shared context for server functions that contains information about the request and middleware state.
///
/// You should not construct this directly inside components or server functions. Instead use [`server_context()`] to get the server context from the current request.
///
/// # Example
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// #[server]
/// async fn read_headers() -> Result<(), ServerFnError> {
///     let server_context = server_context();
///     let headers: http::HeaderMap = server_context.extract().await?;
///     println!("{:?}", headers);
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct DioxusServerContext {
    shared_context: Arc<RwLock<SendSyncAnyMap>>,
    response_parts_modified: Arc<AtomicResponsePartsModified>,
    response_parts: Arc<RwLock<http::response::Parts>>,
    pub(crate) parts: Arc<RwLock<http::request::Parts>>,
    response_sent: Arc<std::sync::atomic::AtomicBool>,
}

enum ContextType {
    Factory(Box<dyn Fn() -> Box<dyn Any> + Send + Sync>),
    Value(Box<dyn Any + Send + Sync>),
}

impl ContextType {
    fn downcast<T: Clone + 'static>(&self) -> Option<T> {
        match self {
            ContextType::Value(value) => value.downcast_ref::<T>().cloned(),
            ContextType::Factory(factory) => factory().downcast::<T>().ok().map(|v| *v),
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for DioxusServerContext {
    fn default() -> Self {
        Self {
            shared_context: Arc::new(RwLock::new(HashMap::new())),
            response_parts_modified: Arc::new(AtomicResponsePartsModified::new()),
            response_parts: Arc::new(RwLock::new(
                http::response::Response::new(()).into_parts().0,
            )),
            parts: Arc::new(RwLock::new(http::request::Request::new(()).into_parts().0)),
            response_sent: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
}

mod server_fn_impl {
    use super::*;
    use parking_lot::{MappedRwLockWriteGuard, RwLockReadGuard, RwLockWriteGuard};
    use std::any::{Any, TypeId};

    impl DioxusServerContext {
        /// Create a new server context from a request
        pub fn new(parts: http::request::Parts) -> Self {
            Self {
                parts: Arc::new(RwLock::new(parts)),
                shared_context: Arc::new(RwLock::new(SendSyncAnyMap::new())),
                response_parts_modified: Arc::new(AtomicResponsePartsModified::new()),
                response_parts: std::sync::Arc::new(RwLock::new(
                    http::response::Response::new(()).into_parts().0,
                )),
                response_sent: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            }
        }

        /// Create a server context from a shared parts
        #[allow(unused)]
        pub(crate) fn from_shared_parts(parts: Arc<RwLock<http::request::Parts>>) -> Self {
            Self {
                parts,
                shared_context: Arc::new(RwLock::new(SendSyncAnyMap::new())),
                response_parts_modified: Arc::new(AtomicResponsePartsModified::new()),
                response_parts: std::sync::Arc::new(RwLock::new(
                    http::response::Response::new(()).into_parts().0,
                )),
                response_sent: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            }
        }

        /// Clone a value from the shared server context. If you are using [`DioxusRouterExt`](crate::prelude::DioxusRouterExt), any values you insert into
        /// the launch context will also be available in the server context.
        ///
        /// Example:
        /// ```rust, no_run
        /// use dioxus::prelude::*;
        ///
        /// LaunchBuilder::new()
        ///     // You can provide context to your whole app (including server functions) with the `with_context` method on the launch builder
        ///     .with_context(server_only! {
        ///         1234567890u32
        ///     })
        ///     .launch(app);
        ///
        /// #[server]
        /// async fn read_context() -> Result<u32, ServerFnError> {
        ///     // You can extract values from the server context with the `extract` function
        ///     let FromContext(value) = extract().await?;
        ///     Ok(value)
        /// }
        ///
        /// fn app() -> Element {
        ///     let future = use_resource(read_context);
        ///     rsx! {
        ///         h1 { "{future:?}" }
        ///     }
        /// }
        /// ```
        pub fn get<T: Any + Send + Sync + Clone + 'static>(&self) -> Option<T> {
            self.shared_context
                .read()
                .get(&TypeId::of::<T>())
                .map(|v| v.downcast::<T>().unwrap())
        }

        /// Insert a value into the shared server context
        pub fn insert<T: Any + Send + Sync + 'static>(&self, value: T) {
            self.insert_any(Box::new(value));
        }

        /// Insert a boxed `Any` value into the shared server context
        pub fn insert_any(&self, value: Box<dyn Any + Send + Sync + 'static>) {
            self.shared_context
                .write()
                .insert((*value).type_id(), ContextType::Value(value));
        }

        /// Insert a factory that creates a non-sync value for the shared server context
        pub fn insert_factory<F, T>(&self, value: F)
        where
            F: Fn() -> T + Send + Sync + 'static,
            T: 'static,
        {
            self.shared_context.write().insert(
                TypeId::of::<T>(),
                ContextType::Factory(Box::new(move || Box::new(value()))),
            );
        }

        /// Insert a boxed factory that creates a non-sync value for the shared server context
        pub fn insert_boxed_factory(&self, value: Box<dyn Fn() -> Box<dyn Any> + Send + Sync>) {
            self.shared_context
                .write()
                .insert((*value()).type_id(), ContextType::Factory(value));
        }

        /// Get the response parts from the server context
        ///
        #[doc = include_str!("../docs/request_origin.md")]
        ///
        /// # Example
        ///
        /// ```rust, no_run
        /// # use dioxus::prelude::*;
        /// #[server]
        /// async fn set_headers() -> Result<(), ServerFnError> {
        ///     let server_context = server_context();
        ///     let response_parts = server_context.response_parts();
        ///     let cookies = response_parts
        ///         .headers
        ///         .get("Cookie")
        ///         .ok_or_else(|| ServerFnError::new("failed to find Cookie header in the response"))?;
        ///     println!("{:?}", cookies);
        ///     Ok(())
        /// }
        /// ```
        pub fn response_parts(&self) -> RwLockReadGuard<'_, http::response::Parts> {
            self.response_parts.read()
        }

        /// Get the headers from the server context mutably
        ///
        #[doc = include_str!("../docs/request_origin.md")]
        ///
        /// # Example
        ///
        /// ```rust, no_run
        /// # use dioxus::prelude::*;
        /// #[server]
        /// async fn set_headers() -> Result<(), ServerFnError> {
        ///     let server_context = server_context();
        ///     server_context.headers_mut()
        ///         .insert("Cookie", http::HeaderValue::from_static("dioxus=fullstack"));
        ///     Ok(())
        /// }
        /// ```
        pub fn headers_mut(&self) -> MappedRwLockWriteGuard<'_, http::HeaderMap> {
            self.response_parts_modified
                .set(ResponsePartsModified::Headers);
            RwLockWriteGuard::map(self.response_parts_mut(), |parts| &mut parts.headers)
        }

        /// Get the status from the server context mutably
        ///
        #[doc = include_str!("../docs/request_origin.md")]
        ///
        /// # Example
        ///
        /// ```rust, no_run
        /// # use dioxus::prelude::*;
        /// #[server]
        /// async fn set_status() -> Result<(), ServerFnError> {
        ///     let server_context = server_context();
        ///     *server_context.status_mut() = http::StatusCode::INTERNAL_SERVER_ERROR;
        ///     Ok(())
        /// }
        /// ```
        pub fn status_mut(&self) -> MappedRwLockWriteGuard<'_, http::StatusCode> {
            self.response_parts_modified
                .set(ResponsePartsModified::Status);
            RwLockWriteGuard::map(self.response_parts_mut(), |parts| &mut parts.status)
        }

        /// Get the version from the server context mutably
        ///
        #[doc = include_str!("../docs/request_origin.md")]
        ///
        /// # Example
        ///
        /// ```rust, no_run
        /// # use dioxus::prelude::*;
        /// #[server]
        /// async fn set_version() -> Result<(), ServerFnError> {
        ///     let server_context = server_context();
        ///     *server_context.version_mut() = http::Version::HTTP_2;
        ///     Ok(())
        /// }
        /// ```
        pub fn version_mut(&self) -> MappedRwLockWriteGuard<'_, http::Version> {
            self.response_parts_modified
                .set(ResponsePartsModified::Version);
            RwLockWriteGuard::map(self.response_parts_mut(), |parts| &mut parts.version)
        }

        /// Get the extensions from the server context mutably
        ///
        #[doc = include_str!("../docs/request_origin.md")]
        ///
        /// # Example
        ///
        /// ```rust, no_run
        /// # use dioxus::prelude::*;
        /// #[server]
        /// async fn set_version() -> Result<(), ServerFnError> {
        ///     let server_context = server_context();
        ///     *server_context.version_mut() = http::Version::HTTP_2;
        ///     Ok(())
        /// }
        /// ```
        pub fn extensions_mut(&self) -> MappedRwLockWriteGuard<'_, http::Extensions> {
            self.response_parts_modified
                .set(ResponsePartsModified::Extensions);
            RwLockWriteGuard::map(self.response_parts_mut(), |parts| &mut parts.extensions)
        }

        /// Get the response parts mutably. This does not track what parts have been written to so it should not be exposed publicly.
        fn response_parts_mut(&self) -> RwLockWriteGuard<'_, http::response::Parts> {
            if self
                .response_sent
                .load(std::sync::atomic::Ordering::Relaxed)
            {
                tracing::error!("Attempted to modify the request after the first frame of the response has already been sent. \
                You can read the response, but modifying the response will not change the response that the client has already received. \
                Try modifying the response before the suspense boundary above the router is resolved.");
            }
            self.response_parts.write()
        }

        /// Get the request parts
        ///
        #[doc = include_str!("../docs/request_origin.md")]
        ///
        /// # Example
        ///
        /// ```rust, no_run
        /// # use dioxus::prelude::*;
        /// #[server]
        /// async fn read_headers() -> Result<(), ServerFnError> {
        ///     let server_context = server_context();
        ///     let request_parts = server_context.request_parts();
        ///     let id: &i32 = request_parts
        ///         .extensions
        ///         .get()
        ///         .ok_or_else(|| ServerFnError::new("failed to find i32 extension in the request"))?;
        ///     println!("{:?}", id);
        ///     Ok(())
        /// }
        /// ```
        pub fn request_parts(&self) -> parking_lot::RwLockReadGuard<'_, http::request::Parts> {
            self.parts.read()
        }

        /// Get the request parts mutably
        ///
        #[doc = include_str!("../docs/request_origin.md")]
        ///
        /// # Example
        ///
        /// ```rust, no_run
        /// # use dioxus::prelude::*;
        /// #[server]
        /// async fn read_headers() -> Result<(), ServerFnError> {
        ///     let server_context = server_context();
        ///     let id: i32 = server_context.request_parts_mut()
        ///         .extensions
        ///         .remove()
        ///         .ok_or_else(|| ServerFnError::new("failed to find i32 extension in the request"))?;
        ///     println!("{:?}", id);
        ///     Ok(())
        /// }
        /// ```
        pub fn request_parts_mut(&self) -> parking_lot::RwLockWriteGuard<'_, http::request::Parts> {
            self.parts.write()
        }

        /// Extract part of the request.
        ///
        #[doc = include_str!("../docs/request_origin.md")]
        ///
        /// # Example
        ///
        /// ```rust, no_run
        /// # use dioxus::prelude::*;
        /// #[server]
        /// async fn read_headers() -> Result<(), ServerFnError> {
        ///     let server_context = server_context();
        ///     let headers: http::HeaderMap = server_context.extract().await?;
        ///     println!("{:?}", headers);
        ///     Ok(())
        /// }
        /// ```
        pub async fn extract<M, T: FromServerContext<M>>(&self) -> Result<T, T::Rejection> {
            T::from_request(self).await
        }

        /// Copy the response parts to a response and mark this server context as sent
        pub(crate) fn send_response<B>(&self, response: &mut http::response::Response<B>) {
            self.response_sent
                .store(true, std::sync::atomic::Ordering::Relaxed);
            let parts = self.response_parts.read();

            if self
                .response_parts_modified
                .is_modified(ResponsePartsModified::Headers)
            {
                let mut_headers = response.headers_mut();
                for (key, value) in parts.headers.iter() {
                    mut_headers.insert(key, value.clone());
                }
            }
            if self
                .response_parts_modified
                .is_modified(ResponsePartsModified::Status)
            {
                *response.status_mut() = parts.status;
            }
            if self
                .response_parts_modified
                .is_modified(ResponsePartsModified::Version)
            {
                *response.version_mut() = parts.version;
            }
            if self
                .response_parts_modified
                .is_modified(ResponsePartsModified::Extensions)
            {
                response.extensions_mut().extend(parts.extensions.clone());
            }
        }
    }
}

#[test]
fn server_context_as_any_map() {
    let parts = http::Request::new(()).into_parts().0;
    let server_context = DioxusServerContext::new(parts);
    server_context.insert_boxed_factory(Box::new(|| Box::new(1234u32)));
    assert_eq!(server_context.get::<u32>().unwrap(), 1234u32);
}

std::thread_local! {
    pub(crate) static SERVER_CONTEXT: std::cell::RefCell<Box<DioxusServerContext>> = Default::default();
}

/// Get information about the current server request.
///
/// This function will only provide the current server context if it is called from a server function or on the server rendering a request.
pub fn server_context() -> DioxusServerContext {
    SERVER_CONTEXT.with(|ctx| *ctx.borrow().clone())
}

/// Extract some part from the current server request.
///
/// This function will only provide the current server context if it is called from a server function or on the server rendering a request.
pub async fn extract<E: FromServerContext<I>, I>() -> Result<E, E::Rejection> {
    E::from_request(&server_context()).await
}

/// Run a function inside of the server context.
pub fn with_server_context<O>(context: DioxusServerContext, f: impl FnOnce() -> O) -> O {
    // before polling the future, we need to set the context
    let prev_context = SERVER_CONTEXT.with(|ctx| ctx.replace(Box::new(context)));
    // poll the future, which may call server_context()
    let result = f();
    // after polling the future, we need to restore the context
    SERVER_CONTEXT.with(|ctx| ctx.replace(prev_context));
    result
}

/// A future that provides the server context to the inner future
#[pin_project::pin_project]
pub struct ProvideServerContext<F: std::future::Future> {
    context: DioxusServerContext,
    #[pin]
    f: F,
}

impl<F: std::future::Future> ProvideServerContext<F> {
    /// Create a new future that provides the server context to the inner future
    pub fn new(f: F, context: DioxusServerContext) -> Self {
        Self { f, context }
    }
}

impl<F: std::future::Future> std::future::Future for ProvideServerContext<F> {
    type Output = F::Output;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();
        let context = this.context.clone();
        with_server_context(context, || this.f.poll(cx))
    }
}

/// A trait for extracting types from the server context
#[async_trait::async_trait]
pub trait FromServerContext<I = ()>: Sized {
    /// The error type returned when extraction fails. This type must implement `std::error::Error`.
    type Rejection;

    /// Extract this type from the server context.
    async fn from_request(req: &DioxusServerContext) -> Result<Self, Self::Rejection>;
}

/// A type was not found in the server context
pub struct NotFoundInServerContext<T: 'static>(std::marker::PhantomData<T>);

impl<T: 'static> std::fmt::Debug for NotFoundInServerContext<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let type_name = std::any::type_name::<T>();
        write!(f, "`{type_name}` not found in server context")
    }
}

impl<T: 'static> std::fmt::Display for NotFoundInServerContext<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let type_name = std::any::type_name::<T>();
        write!(f, "`{type_name}` not found in server context")
    }
}

impl<T: 'static> std::error::Error for NotFoundInServerContext<T> {}

/// Extract a value from the server context provided through the launch builder context or [`DioxusServerContext::insert`]
///
/// Example:
/// ```rust, no_run
/// use dioxus::prelude::*;
///
/// dioxus::LaunchBuilder::new()
///     // You can provide context to your whole app (including server functions) with the `with_context` method on the launch builder
///     .with_context(server_only! {
///         1234567890u32
///     })
///     .launch(app);
///
/// #[server]
/// async fn read_context() -> Result<u32, ServerFnError> {
///     // You can extract values from the server context with the `extract` function
///     let FromContext(value) = extract().await?;
///     Ok(value)
/// }
///
/// fn app() -> Element {
///     let future = use_resource(read_context);
///     rsx! {
///         h1 { "{future:?}" }
///     }
/// }
/// ```
pub struct FromContext<T: std::marker::Send + std::marker::Sync + Clone + 'static>(pub T);

#[async_trait::async_trait]
impl<T: Send + Sync + Clone + 'static> FromServerContext for FromContext<T> {
    type Rejection = NotFoundInServerContext<T>;

    async fn from_request(req: &DioxusServerContext) -> Result<Self, Self::Rejection> {
        Ok(Self(req.get::<T>().ok_or({
            NotFoundInServerContext::<T>(std::marker::PhantomData::<T>)
        })?))
    }
}

/// An adapter for axum extractors for the server context
pub struct Axum;

#[async_trait::async_trait]
impl<I: axum::extract::FromRequestParts<()>> FromServerContext<Axum> for I {
    type Rejection = I::Rejection;

    #[allow(clippy::all)]
    async fn from_request(req: &DioxusServerContext) -> Result<Self, Self::Rejection> {
        let mut lock = req.request_parts_mut();
        I::from_request_parts(&mut lock, &()).await
    }
}
