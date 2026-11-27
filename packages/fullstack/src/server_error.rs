use axum::response::{IntoResponse, Response};
use dioxus_fullstack_core::ServerFnError;
use std::fmt;

/// A [`ServerFnError`] that can be returned from a hand-written axum handler.
///
/// `ServerFnError` lives in `dioxus-fullstack-core`, which does not depend on axum, so it cannot
/// implement [`IntoResponse`] itself. Anything that converts into a `ServerFnError` (including
/// [`HttpError`](crate::HttpError) and `anyhow::Error`) converts into `ServerError`, so `?` works
/// in handlers returning `Result<T, ServerError>`.
///
/// ```rust, no_run
/// # use dioxus_fullstack::{ServerError, ServerFnError};
/// async fn handler() -> Result<String, ServerError> {
///     let value: u32 = "42".parse().map_err(ServerFnError::new)?;
///     Ok(value.to_string())
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerError(pub ServerFnError);

impl ServerError {
    /// Get the inner [`ServerFnError`].
    pub fn into_inner(self) -> ServerFnError {
        self.0
    }
}

impl<E: Into<ServerFnError>> From<E> for ServerError {
    fn from(value: E) -> Self {
        Self(value.into())
    }
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for ServerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        self.0.into_http_response().into_response()
    }
}
