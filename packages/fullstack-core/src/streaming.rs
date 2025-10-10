use crate::{HttpError, ServerFnError};
use axum_core::{extract::FromRequest, response::IntoResponse};
use dioxus_core::{try_consume_context, CapturedError};
use dioxus_signals::{ReadableExt, Signal, WritableExt};
use http::request::Parts;
use http::StatusCode;
use std::{cell::RefCell, rc::Rc};

/// The context provided by dioxus fullstack for server-side rendering.
///
/// This context will only be set on the server during a streaming response.
#[derive(Clone, Debug)]
pub struct FullstackContext {
    current_status: Signal<StreamingStatus>,
    request_headers: Rc<RefCell<http::request::Parts>>,
    route_http_status: Signal<HttpError>,
}

impl PartialEq for FullstackContext {
    fn eq(&self, other: &Self) -> bool {
        self.current_status == other.current_status
            && Rc::ptr_eq(&self.request_headers, &other.request_headers)
    }
}

impl FullstackContext {
    /// Create a new streaming context. You should not need to call this directly. Dioxus fullstack will
    /// provide this context for you.
    pub fn new(parts: Parts) -> Self {
        Self {
            current_status: Signal::new(StreamingStatus::RenderingInitialChunk),
            request_headers: Rc::new(RefCell::new(parts)),
            route_http_status: Signal::new(HttpError {
                status: http::StatusCode::OK,
                message: None,
            }),
        }
    }

    /// Commit the initial chunk of the response. This will be called automatically if you are using the
    /// dioxus router when the suspense boundary above the router is resolved. Otherwise, you will need
    /// to call this manually to start the streaming part of the response.
    ///
    /// Once this method has been called, the http response parts can no longer be modified.
    pub fn commit_initial_chunk(&mut self) {
        self.current_status
            .set(StreamingStatus::InitialChunkCommitted);
    }

    /// Get the current status of the streaming response. This method is reactive and will cause
    /// the current reactive context to rerun when the status changes.
    pub fn streaming_state(&self) -> StreamingStatus {
        *self.current_status.read()
    }

    /// Access the http request parts mutably. This will allow you to modify headers and other parts of the request.
    pub fn parts_mut(&self) -> std::cell::RefMut<'_, http::request::Parts> {
        self.request_headers.borrow_mut()
    }

    /// Extract an axum extractor from the current request. This will always use an empty body for the request,
    /// since it's assumed that rendering the app is done under a `GET` request.
    pub async fn extract<T: FromRequest<(), M>, M>() -> Result<T, ServerFnError> {
        let this = Self::current()
            .ok_or_else(|| ServerFnError::new("No StreamingContext found".to_string()))?;

        let parts = this.request_headers.borrow_mut().clone();
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

    /// Get the current `StreamingContext` if it exists. This will return `None` if called on the client
    /// or outside of a streaming response on the server.
    pub fn current() -> Option<Self> {
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

    /// Set the current HTTP status for the route. This will be used when committing the response
    /// to the client.
    pub fn set_route_http_status(&mut self, status: HttpError) {
        self.route_http_status.set(status);
    }

    /// Commit the current HTTP status for the route. This will be used when committing the response
    /// to the client. The returned `HttpError` can be used to render different UI based on the error.
    pub fn commit_error_status(error: impl Into<CapturedError>) -> HttpError {
        let error = error.into();
        let status = err_to_status_code(&error);
        let http_error = HttpError {
            status,
            message: Some(error.to_string()),
        };

        if let Some(mut ctx) = Self::current() {
            ctx.set_route_http_status(http_error.clone());
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
pub fn err_to_status_code(error: &CapturedError) -> StatusCode {
    if let Some(err) = error.downcast_ref::<ServerFnError>() {
        match err {
            ServerFnError::ServerError {
                code: Some(code), ..
            } => return StatusCode::from_u16(*code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
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
