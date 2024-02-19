use crate::html_storage::HTMLData;
use std::sync::Arc;
use std::sync::RwLock;

/// A shared context for server functions that contains information about the request and middleware state.
/// This allows you to pass data between your server framework and the server functions. This can be used to pass request information or information about the state of the server. For example, you could pass authentication data though this context to your server functions.
///
/// You should not construct this directly inside components. Instead use the `HasServerContext` trait to get the server context from the scope.
#[derive(Clone)]
pub struct DioxusServerContext {
    shared_context: std::sync::Arc<
        std::sync::RwLock<anymap::Map<dyn anymap::any::Any + Send + Sync + 'static>>,
    >,
    response_parts: std::sync::Arc<std::sync::RwLock<http::response::Parts>>,
    pub(crate) parts: Arc<tokio::sync::RwLock<http::request::Parts>>,
    html_data: Arc<RwLock<HTMLData>>,
}

#[allow(clippy::derivable_impls)]
impl Default for DioxusServerContext {
    fn default() -> Self {
        Self {
            shared_context: std::sync::Arc::new(std::sync::RwLock::new(anymap::Map::new())),
            response_parts: std::sync::Arc::new(RwLock::new(
                http::response::Response::new(()).into_parts().0,
            )),
            parts: std::sync::Arc::new(tokio::sync::RwLock::new(
                http::request::Request::new(()).into_parts().0,
            )),
            html_data: Arc::new(RwLock::new(HTMLData::default())),
        }
    }
}

mod server_fn_impl {
    use super::*;
    use std::sync::LockResult;
    use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

    use anymap::{any::Any, Map};
    type SendSyncAnyMap = Map<dyn Any + Send + Sync + 'static>;

    impl DioxusServerContext {
        /// Create a new server context from a request
        pub fn new(parts: impl Into<Arc<tokio::sync::RwLock<http::request::Parts>>>) -> Self {
            Self {
                parts: parts.into(),
                shared_context: Arc::new(RwLock::new(SendSyncAnyMap::new())),
                response_parts: std::sync::Arc::new(RwLock::new(
                    http::response::Response::new(()).into_parts().0,
                )),
                html_data: Arc::new(RwLock::new(HTMLData::default())),
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

        /// Get the response parts from the server context
        pub fn response_parts(&self) -> LockResult<RwLockReadGuard<'_, http::response::Parts>> {
            self.response_parts.read()
        }

        /// Get the response parts from the server context
        pub fn response_parts_mut(
            &self,
        ) -> LockResult<RwLockWriteGuard<'_, http::response::Parts>> {
            self.response_parts.write()
        }

        /// Get the request that triggered:
        /// - The initial SSR render if called from a ScopeState or ServerFn
        /// - The server function to be called if called from a server function after the initial render
        pub fn request_parts(&self) -> tokio::sync::RwLockReadGuard<'_, http::request::Parts> {
            self.parts.blocking_read()
        }

        /// Get the request that triggered:
        /// - The initial SSR render if called from a ScopeState or ServerFn
        /// - The server function to be called if called from a server function after the initial render
        pub fn request_parts_mut(&self) -> tokio::sync::RwLockWriteGuard<'_, http::request::Parts> {
            self.parts.blocking_write()
        }

        /// Extract some part from the request
        pub async fn extract<R: std::error::Error, T: FromServerContext<Rejection = R>>(
            &self,
        ) -> Result<T, R> {
            T::from_request(self).await
        }

        /// Insert some data into the html data store
        pub(crate) fn push_html_data<T: serde::Serialize>(
            &self,
            value: &T,
        ) -> Result<(), PoisonError<RwLockWriteGuard<'_, HTMLData>>> {
            self.html_data.write().map(|mut map| {
                map.push(value);
            })
        }

        /// Get the html data store
        pub(crate) fn html_data(&self) -> LockResult<RwLockReadGuard<'_, HTMLData>> {
            self.html_data.read()
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

pub(crate) fn with_server_context<O>(
    context: Box<DioxusServerContext>,
    f: impl FnOnce() -> O,
) -> (O, Box<DioxusServerContext>) {
    // before polling the future, we need to set the context
    let prev_context = SERVER_CONTEXT.with(|ctx| ctx.replace(context));
    // poll the future, which may call server_context()
    let result = f();
    // after polling the future, we need to restore the context
    (result, SERVER_CONTEXT.with(|ctx| ctx.replace(prev_context)))
}

/// A future that provides the server context to the inner future
#[pin_project::pin_project]
pub struct ProvideServerContext<F: std::future::Future> {
    context: Option<Box<DioxusServerContext>>,
    #[pin]
    f: F,
}

impl<F: std::future::Future> ProvideServerContext<F> {
    /// Create a new future that provides the server context to the inner future
    pub fn new(f: F, context: DioxusServerContext) -> Self {
        Self {
            context: Some(Box::new(context)),
            f,
        }
    }
}

impl<F: std::future::Future> std::future::Future for ProvideServerContext<F> {
    type Output = F::Output;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();
        let context = this.context.take().unwrap();
        let (result, context) = with_server_context(context, || this.f.poll(cx));
        *this.context = Some(context);
        result
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

pub struct FromContext<T: std::marker::Send + std::marker::Sync + Clone + 'static>(pub(crate) T);

#[async_trait::async_trait]
impl<T: Send + Sync + Clone + 'static> FromServerContext for FromContext<T> {
    type Rejection = NotFoundInServerContext<T>;

    async fn from_request(req: &DioxusServerContext) -> Result<Self, Self::Rejection> {
        Ok(Self(req.clone().get::<T>().ok_or({
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

    async fn from_request(req: &DioxusServerContext) -> Result<Self, Self::Rejection> {
        Ok(I::from_request_parts(&mut req.request_parts_mut(), &()).await?)
    }
}
