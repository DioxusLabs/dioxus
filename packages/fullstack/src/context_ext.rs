use axum::extract::FromRequest;
use axum::response::IntoResponse;
use dioxus_fullstack_core::{FullstackContext, ServerFnError};
use std::future::Future;

/// Axum integration for [`FullstackContext`].
pub trait FullstackContextExt {
    /// Extract an axum extractor from the current request.
    ///
    /// The body of the request is always empty when using this method, as the body can only be consumed once in the server
    /// function extractors.
    fn extract<T: FromRequest<FullstackContext, M>, M>()
    -> impl Future<Output = Result<T, ServerFnError>> + Send;
}

impl FullstackContextExt for FullstackContext {
    fn extract<T: FromRequest<FullstackContext, M>, M>()
    -> impl Future<Output = Result<T, ServerFnError>> + Send {
        async move {
            let this = Self::current().unwrap_or_else(|| {
                // Create a dummy context if one doesn't exist, making the function usable outside of a request context
                FullstackContext::new(
                    http::Request::builder()
                        .method("GET")
                        .uri("/")
                        .header("X-Dummy-Header", "true")
                        .body(())
                        .unwrap()
                        .into_parts()
                        .0,
                )
            });

            let parts = this.parts_mut().clone();
            let request = axum::extract::Request::from_parts(parts, Default::default());
            let response = match T::from_request(request, &this).await {
                Ok(res) => return Ok(res),
                Err(err) => err.into_response(),
            };

            let (parts, body) = response.into_parts();
            let body = axum::body::to_bytes(body, usize::MAX)
                .await
                .unwrap_or_default();
            Err(ServerFnError::from_http_response(
                http::Response::from_parts(parts, body),
            ))
        }
    }
}

/// Extract an axum extractor from the current request.
#[deprecated(note = "Use FullstackContext::extract instead", since = "0.7.0")]
pub fn extract<T: FromRequest<FullstackContext, M>, M>()
-> impl Future<Output = Result<T, ServerFnError>> + Send {
    FullstackContext::extract::<T, M>()
}
