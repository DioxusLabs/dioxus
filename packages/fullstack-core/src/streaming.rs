use crate::{HttpError, ServerFnError};
use axum_core::extract::FromRequest;
use axum_core::response::IntoResponse;
use dioxus_core::{CapturedError, ReactiveContext};
use http::StatusCode;
use http::{request::Parts, HeaderMap};
use parking_lot::RwLock;
use std::collections::HashSet;
use std::fmt::Debug;
use std::sync::Arc;

/// The context provided by dioxus fullstack for server-side rendering.
///
/// This context will only be set on the server during the initial streaming response
/// and inside server functions.
#[derive(Clone, Debug)]
pub struct FullstackContext {
    // We expose the lock for request headers directly so it needs to be in a separate lock
    request_headers: Arc<RwLock<http::request::Parts>>,

    // The rest of the fields are only held internally, so we can group them together
    lock: Arc<RwLock<FullstackContextInner>>,
}

// `FullstackContext` is always set when either
// 1. rendering the app via SSR
// 2. handling a server function request
tokio::task_local! {
    static FULLSTACK_CONTEXT: FullstackContext;
}

pub struct FullstackContextInner {
    current_status: StreamingStatus,
    current_status_subscribers: HashSet<ReactiveContext>,
    response_headers: Option<HeaderMap>,
    route_http_status: HttpError,
    route_http_status_subscribers: HashSet<ReactiveContext>,
}

impl Debug for FullstackContextInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FullstackContextInner")
            .field("current_status", &self.current_status)
            .field("response_headers", &self.response_headers)
            .field("route_http_status", &self.route_http_status)
            .finish()
    }
}

impl PartialEq for FullstackContext {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.lock, &other.lock)
            && Arc::ptr_eq(&self.request_headers, &other.request_headers)
    }
}

impl FullstackContext {
    /// Create a new streaming context. You should not need to call this directly. Dioxus fullstack will
    /// provide this context for you.
    pub fn new(parts: Parts) -> Self {
        Self {
            request_headers: RwLock::new(parts).into(),
            lock: RwLock::new(FullstackContextInner {
                current_status: StreamingStatus::RenderingInitialChunk,
                current_status_subscribers: Default::default(),
                route_http_status: HttpError {
                    status: http::StatusCode::OK,
                    message: None,
                },
                route_http_status_subscribers: Default::default(),
                response_headers: Some(HeaderMap::new()),
            })
            .into(),
        }
    }

    /// Commit the initial chunk of the response. This will be called automatically if you are using the
    /// dioxus router when the suspense boundary above the router is resolved. Otherwise, you will need
    /// to call this manually to start the streaming part of the response.
    ///
    /// Once this method has been called, the http response parts can no longer be modified.
    pub fn commit_initial_chunk(&mut self) {
        let mut lock = self.lock.write();
        lock.current_status = StreamingStatus::InitialChunkCommitted;

        // The key type is mutable, but the hash is stable through mutations because we hash by pointer
        #[allow(clippy::mutable_key_type)]
        let subscribers = std::mem::take(&mut lock.current_status_subscribers);
        for subscriber in subscribers {
            subscriber.mark_dirty();
        }
    }

    /// Get the current status of the streaming response. This method is reactive and will cause
    /// the current reactive context to rerun when the status changes.
    pub fn streaming_state(&self) -> StreamingStatus {
        let mut lock = self.lock.write();
        // Register the current reactive context as a subscriber to changes in the streaming status
        if let Some(ctx) = ReactiveContext::current() {
            lock.current_status_subscribers.insert(ctx);
        }
        lock.current_status
    }

    /// Access the http request parts mutably. This will allow you to modify headers and other parts of the request.
    pub fn parts_mut(&self) -> parking_lot::RwLockWriteGuard<'_, http::request::Parts> {
        self.request_headers.write()
    }

    /// Run a future within the scope of this FullstackContext.
    pub async fn scope<F, R>(self, fut: F) -> R
    where
        F: std::future::Future<Output = R>,
    {
        FULLSTACK_CONTEXT.scope(self, fut).await
    }

    /// Extract an extension from the current request.
    pub fn extension<T: Clone + Send + Sync + 'static>(&self) -> Option<T> {
        let lock = self.request_headers.read();
        lock.extensions.get::<T>().cloned()
    }

    /// Extract an axum extractor from the current request.
    ///
    /// The body of the request is always empty when using this method, as the body can only be consumed once in the server
    /// function extractors.
    pub async fn extract<T: FromRequest<Self, M>, M>() -> Result<T, ServerFnError> {
        let this = Self::current().unwrap_or_else(|| {
            // Create a dummy context if one doesn't exist, making the function usable outside of a request context
            FullstackContext::new(
                axum_core::extract::Request::builder()
                    .method("GET")
                    .uri("/")
                    .header("X-Dummy-Header", "true")
                    .body(())
                    .unwrap()
                    .into_parts()
                    .0,
            )
        });

        let parts = this.request_headers.read().clone();
        let request = axum_core::extract::Request::from_parts(parts, Default::default());
        match T::from_request(request, &this).await {
            Ok(res) => Ok(res),
            Err(err) => {
                let resp = err.into_response();
                Err(ServerFnError::from_axum_response(resp).await)
            }
        }
    }

    /// Get the current `FullstackContext` if it exists. This will return `None` if called on the client
    /// or outside of a streaming response on the server or server function.
    pub fn current() -> Option<Self> {
        // Try to get the context from the task local (for server functions)
        if let Ok(context) = FULLSTACK_CONTEXT.try_get() {
            return Some(context);
        }

        // Otherwise, try to get it from the dioxus runtime context (for streaming SSR)
        if let Some(rt) = dioxus_core::Runtime::try_current() {
            let id = rt.try_current_scope_id()?;
            if let Some(ctx) = rt.consume_context::<FullstackContext>(id) {
                return Some(ctx);
            }
        }

        None
    }

    /// Get the current HTTP status for the route. This will default to 200 OK, but can be modified
    /// by calling `FullstackContext::commit_error_status` with an error.
    pub fn current_http_status(&self) -> HttpError {
        let mut lock = self.lock.write();
        // Register the current reactive context as a subscriber to changes in the http status
        if let Some(ctx) = ReactiveContext::current() {
            lock.route_http_status_subscribers.insert(ctx);
        }
        lock.route_http_status.clone()
    }

    pub fn set_current_http_status(&mut self, status: HttpError) {
        let mut lock = self.lock.write();
        lock.route_http_status = status;
        // The key type is mutable, but the hash is stable through mutations because we hash by pointer
        #[allow(clippy::mutable_key_type)]
        let subscribers = std::mem::take(&mut lock.route_http_status_subscribers);
        for subscriber in subscribers {
            subscriber.mark_dirty();
        }
    }

    /// Add a header to the response. This will be sent to the client when the response is committed.
    pub fn add_response_header(
        &self,
        key: impl Into<http::header::HeaderName>,
        value: impl Into<http::header::HeaderValue>,
    ) {
        let mut lock = self.lock.write();
        if let Some(headers) = lock.response_headers.as_mut() {
            headers.insert(key.into(), value.into());
        }
    }

    /// Take the response headers out of the context. This will leave the context without any headers,
    /// so it should only be called once when the response is being committed.
    pub fn take_response_headers(&self) -> Option<HeaderMap> {
        let mut lock = self.lock.write();
        lock.response_headers.take()
    }

    /// Set the current HTTP status for the route. This will be used when committing the response
    /// to the client.
    pub fn commit_http_status(status: StatusCode, message: Option<String>) {
        if let Some(mut ctx) = Self::current() {
            ctx.set_current_http_status(HttpError { status, message });
        }
    }

    /// Commit the CapturedError as the current HTTP status for the route.
    /// This will attempt to downcast the error to known types and set the appropriate
    /// status code. If the error type is unknown, it will default to
    /// `StatusCode::INTERNAL_SERVER_ERROR`.
    pub fn commit_error_status(error: impl Into<CapturedError>) -> HttpError {
        let error = error.into();
        let status = status_code_from_error(&error);
        let http_error = HttpError {
            status,
            message: Some(error.to_string()),
        };

        if let Some(mut ctx) = Self::current() {
            ctx.set_current_http_status(http_error.clone());
        }

        http_error
    }
}

/// The status of the streaming response
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StreamingStatus {
    /// The initial chunk is still being rendered. The http response parts can still be modified at this point.
    RenderingInitialChunk,

    /// The initial chunk has been committed and the response is now streaming. The http response parts
    /// have already been sent to the client and can no longer be modified.
    InitialChunkCommitted,
}

/// Commit the initial chunk of the response. This will be called automatically if you are using the
/// dioxus router when the suspense boundary above the router is resolved. Otherwise, you will need
/// to call this manually to start the streaming part of the response.
///
/// On the client, this will do nothing.
///
/// # Example
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// # use dioxus_fullstack_core::*;
/// # fn Children() -> Element { unimplemented!() }
/// fn App() -> Element {
///     // This will start streaming immediately after the current render is complete.
///     use_hook(commit_initial_chunk);
///
///     rsx! { Children {} }
/// }
/// ```
pub fn commit_initial_chunk() {
    crate::history::finalize_route();
    if let Some(mut streaming) = FullstackContext::current() {
        streaming.commit_initial_chunk();
    }
}

/// Extract an axum extractor from the current request.
#[deprecated(note = "Use FullstackContext::extract instead", since = "0.7.0")]
pub fn extract<T: FromRequest<FullstackContext, M>, M>(
) -> impl std::future::Future<Output = Result<T, ServerFnError>> {
    FullstackContext::extract::<T, M>()
}

/// Get the current status of the streaming response. This method is reactive and will cause
/// the current reactive context to rerun when the status changes.
///
/// On the client, this will always return `StreamingStatus::InitialChunkCommitted`.
///
/// # Example
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// # use dioxus_fullstack_core::*;
/// #[component]
/// fn MetaTitle(title: String) -> Element {
///     // If streaming has already started, warn the user that the meta tag will not show
///     // up in the initial chunk.
///     use_hook(|| {
///         if current_status() == StreamingStatus::InitialChunkCommitted {
///            dioxus::logger::tracing::warn!("Since `MetaTitle` was rendered after the initial chunk was committed, the meta tag will not show up in the head without javascript enabled.");
///         }
///     });
///
///     rsx! { meta { property: "og:title", content: title } }
/// }
/// ```
pub fn current_status() -> StreamingStatus {
    if let Some(streaming) = FullstackContext::current() {
        streaming.streaming_state()
    } else {
        StreamingStatus::InitialChunkCommitted
    }
}

/// Convert a `CapturedError` into an appropriate HTTP status code.
///
/// This will attempt to downcast the error to known types and return a corresponding status code.
/// If the error type is unknown, it will default to `StatusCode::INTERNAL_SERVER_ERROR`.
pub fn status_code_from_error(error: &CapturedError) -> StatusCode {
    if let Some(err) = error.downcast_ref::<ServerFnError>() {
        match err {
            ServerFnError::ServerError { code, .. } => {
                return StatusCode::from_u16(*code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
            }
            _ => return StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    if let Some(err) = error.downcast_ref::<StatusCode>() {
        return *err;
    }

    if let Some(err) = error.downcast_ref::<HttpError>() {
        return err.status;
    }

    StatusCode::INTERNAL_SERVER_ERROR
}
