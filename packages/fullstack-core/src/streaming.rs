use crate::{HttpError, ServerFnError};
use axum_core::extract::FromRequestParts;
use axum_core::{extract::FromRequest, response::IntoResponse};
use dioxus_core::{try_consume_context, CapturedError, ReactiveContext};
use http::StatusCode;
use http::{request::Parts, HeaderMap};
use parking_lot::RwLock;
use std::collections::HashSet;
use std::fmt::Debug;
use std::sync::Arc;

tokio::task_local! {
    static FULLSTACK_CONTEXT: FullstackContext;
}

/// The context provided by dioxus fullstack for server-side rendering.
///
/// This context will only be set on the server during the initial streaming response
/// and inside server functions.
#[derive(Clone)]
pub struct FullstackContext {
    current_status: Arc<RwLock<StreamingStatus>>,
    current_status_subscribers: Arc<RwLock<HashSet<ReactiveContext>>>,
    request_headers: Arc<RwLock<http::request::Parts>>,
    response_headers: Arc<RwLock<Option<HeaderMap>>>,
    route_http_status: Arc<RwLock<HttpError>>,
    route_http_status_subscribers: Arc<RwLock<HashSet<ReactiveContext>>>,
}

impl Debug for FullstackContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FullstackContext")
            .field("current_status", &self.current_status)
            .field("request_headers", &self.request_headers)
            .field("route_http_status", &self.route_http_status)
            .finish()
    }
}

impl PartialEq for FullstackContext {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.current_status, &other.current_status)
            && Arc::ptr_eq(&self.request_headers, &other.request_headers)
            && Arc::ptr_eq(&self.route_http_status, &other.route_http_status)
            && Arc::ptr_eq(&self.response_headers, &other.response_headers)
            && Arc::ptr_eq(
                &self.current_status_subscribers,
                &other.current_status_subscribers,
            )
            && Arc::ptr_eq(
                &self.route_http_status_subscribers,
                &other.route_http_status_subscribers,
            )
    }
}

impl FullstackContext {
    /// Create a new streaming context. You should not need to call this directly. Dioxus fullstack will
    /// provide this context for you.
    pub fn new(parts: Parts) -> Self {
        Self {
            current_status: Arc::new(RwLock::new(StreamingStatus::RenderingInitialChunk)),
            current_status_subscribers: Default::default(),
            request_headers: RwLock::new(parts).into(),
            route_http_status: Arc::new(RwLock::new(HttpError {
                status: http::StatusCode::OK,
                message: None,
            })),
            route_http_status_subscribers: Default::default(),
            response_headers: RwLock::new(Some(HeaderMap::new())).into(),
        }
    }

    /// Commit the initial chunk of the response. This will be called automatically if you are using the
    /// dioxus router when the suspense boundary above the router is resolved. Otherwise, you will need
    /// to call this manually to start the streaming part of the response.
    ///
    /// Once this method has been called, the http response parts can no longer be modified.
    pub fn commit_initial_chunk(&mut self) {
        *self.current_status.write() = StreamingStatus::InitialChunkCommitted;
        let subscribers = std::mem::take(&mut *self.current_status_subscribers.write());
        for subscriber in subscribers {
            subscriber.mark_dirty();
        }
    }

    /// Get the current status of the streaming response. This method is reactive and will cause
    /// the current reactive context to rerun when the status changes.
    pub fn streaming_state(&self) -> StreamingStatus {
        *self.current_status.read()
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

    /// Extract an axum extractor from the current request.
    pub async fn extract<T: FromRequestParts<()>>() -> Result<T, ServerFnError> {
        let this = Self::current()
            .ok_or_else(|| ServerFnError::new("No FullstackContext found".to_string()))?;

        let parts = this.request_headers.read().clone();
        let request =
            axum_core::extract::Request::from_parts(parts, axum_core::body::Body::empty());
        match T::from_request(request, &()).await {
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
        if let Ok(context) = FULLSTACK_CONTEXT.try_get() {
            return Some(context);
        }

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
        self.route_http_status.read().clone()
    }

    pub fn set_current_http_status(&mut self, status: HttpError) {
        *self.route_http_status.write() = status;
        let subscribers = std::mem::take(&mut *self.route_http_status_subscribers.write());
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
        if let Some(headers) = self.response_headers.write().as_mut() {
            headers.insert(key.into(), value.into());
        }
    }

    /// Take the response headers out of the context. This will leave the context without any headers,
    /// so it should only be called once when the response is being committed.
    pub fn take_response_headers(&self) -> Option<HeaderMap> {
        self.response_headers.write().take()
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
    if let Some(mut streaming) = try_consume_context::<FullstackContext>() {
        streaming.commit_initial_chunk();
    }
}

/// Extract an axum extractor from the current request.
pub fn extract<T: FromRequestParts<()>>(
) -> impl std::future::Future<Output = Result<T, ServerFnError>> {
    FullstackContext::extract::<T>()
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
    if let Some(streaming) = try_consume_context::<FullstackContext>() {
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
