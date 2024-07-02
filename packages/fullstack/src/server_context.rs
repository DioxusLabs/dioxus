use parking_lot::RwLock;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

type SendSyncAnyMap =
    std::collections::HashMap<std::any::TypeId, Box<dyn Any + Send + Sync + 'static>>;

/// A shared context for server functions that contains information about the request and middleware state.
/// This allows you to pass data between your server framework and the server functions. This can be used to pass request information or information about the state of the server. For example, you could pass authentication data though this context to your server functions.
///
/// You should not construct this directly inside components. Instead use the `HasServerContext` trait to get the server context from the scope.
#[derive(Clone)]
pub struct DioxusServerContext {
    shared_context: std::sync::Arc<RwLock<SendSyncAnyMap>>,
    response_parts: std::sync::Arc<RwLock<http::response::Parts>>,
    pub(crate) parts: Arc<RwLock<http::request::Parts>>,
}

#[allow(clippy::derivable_impls)]
impl Default for DioxusServerContext {
    fn default() -> Self {
        Self {
            shared_context: std::sync::Arc::new(RwLock::new(HashMap::new())),
            response_parts: std::sync::Arc::new(RwLock::new(
                http::response::Response::new(()).into_parts().0,
            )),
            parts: std::sync::Arc::new(RwLock::new(http::request::Request::new(()).into_parts().0)),
        }
    }
}

mod server_fn_impl {
    use super::*;
    use parking_lot::{RwLockReadGuard, RwLockWriteGuard};
    use std::any::{Any, TypeId};

    impl DioxusServerContext {
        /// Create a new server context from a request
        pub fn new(parts: http::request::Parts) -> Self {
            Self {
                parts: Arc::new(RwLock::new(parts)),
                shared_context: Arc::new(RwLock::new(SendSyncAnyMap::new())),
                response_parts: std::sync::Arc::new(RwLock::new(
                    http::response::Response::new(()).into_parts().0,
                )),
            }
        }

        /// Create a server context from a shared parts
        #[allow(unused)]
        pub(crate) fn from_shared_parts(parts: Arc<RwLock<http::request::Parts>>) -> Self {
            Self {
                parts,
                shared_context: Arc::new(RwLock::new(SendSyncAnyMap::new())),
                response_parts: std::sync::Arc::new(RwLock::new(
                    http::response::Response::new(()).into_parts().0,
                )),
            }
        }

        /// Clone a value from the shared server context
        pub fn get<T: Any + Send + Sync + Clone + 'static>(&self) -> Option<T> {
            self.shared_context
                .read()
                .get(&TypeId::of::<T>())
                .map(|v| v.downcast_ref::<T>().unwrap().clone())
        }

        /// Insert a value into the shared server context
        pub fn insert<T: Any + Send + Sync + 'static>(&self, value: T) {
            self.shared_context
                .write()
                .insert(TypeId::of::<T>(), Box::new(value));
        }

        /// Insert a Boxed `Any` value into the shared server context
        pub fn insert_any(&self, value: Box<dyn Any + Send + Sync>) {
            self.shared_context
                .write()
                .insert((*value).type_id(), value);
        }

        /// Get the response parts from the server context
        pub fn response_parts(&self) -> RwLockReadGuard<'_, http::response::Parts> {
            self.response_parts.read()
        }

        /// Get the response parts from the server context
        pub fn response_parts_mut(&self) -> RwLockWriteGuard<'_, http::response::Parts> {
            self.response_parts.write()
        }

        /// Get the request that triggered:
        /// - The initial SSR render if called from a ScopeState or ServerFn
        /// - The server function to be called if called from a server function after the initial render
        pub fn request_parts(&self) -> parking_lot::RwLockReadGuard<'_, http::request::Parts> {
            self.parts.read()
        }

        /// Get the request that triggered:
        /// - The initial SSR render if called from a ScopeState or ServerFn
        /// - The server function to be called if called from a server function after the initial render
        pub fn request_parts_mut(&self) -> parking_lot::RwLockWriteGuard<'_, http::request::Parts> {
            self.parts.write()
        }

        /// Extract some part from the request
        pub async fn extract<R: std::error::Error, T: FromServerContext<Rejection = R>>(
            &self,
        ) -> Result<T, R> {
            T::from_request(self).await
        }
    }
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
    type Rejection: std::error::Error;

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

#[cfg(feature = "axum")]
#[cfg_attr(docsrs, doc(cfg(feature = "axum")))]
/// An adapter for axum extractors for the server context
pub struct Axum;

#[cfg(feature = "axum")]
#[async_trait::async_trait]
impl<
        I: axum::extract::FromRequestParts<(), Rejection = R>,
        R: axum::response::IntoResponse + std::error::Error,
    > FromServerContext<Axum> for I
{
    type Rejection = R;

    #[allow(clippy::all)]
    async fn from_request(req: &DioxusServerContext) -> Result<Self, Self::Rejection> {
        let mut lock = req.request_parts_mut();
        I::from_request_parts(&mut lock, &()).await
    }
}
