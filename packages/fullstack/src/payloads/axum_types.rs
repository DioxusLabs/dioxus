use super::*;
use crate::{ClientResponse, FromResponse};
pub use axum::extract::Json;
use axum::response::Html;
use dioxus_fullstack_core::{RequestError, ServerFnError};
use futures::StreamExt;
use std::future::Future;

impl<T: From<String>> FromResponse for Html<T> {
    fn from_response(res: ClientResponse) -> impl Future<Output = Result<Self, ServerFnError>> {
        async move {
            let content = res.text().await?;
            Ok(Html(content.into()))
        }
    }
}

impl<T> IntoRequest for Json<T>
where
    T: Serialize + 'static,
{
    fn into_request(self, request: ClientRequest) -> impl Future<Output = ClientResult> + 'static {
        async move { request.send_json(&self.0).await }
    }
}

impl<T: DeserializeOwned> FromResponse for Json<T> {
    fn from_response(res: ClientResponse) -> impl Future<Output = Result<Self, ServerFnError>> {
        async move {
            let data = res.json::<T>().await?;
            Ok(Json(data))
        }
    }
}

/// Implementation of `FromResponse` for `axum::response::Response`.
///
/// This allows converting a `ClientResponse` (from a client-side HTTP request)
/// into an `axum::Response` for server-side handling. The response's status,
/// headers, and body are transferred from the client response to the axum response.
impl FromResponse for axum::response::Response {
    fn from_response(res: ClientResponse) -> impl Future<Output = Result<Self, ServerFnError>> {
        async move {
            let parts = res.make_parts();
            let body = axum::body::Body::from_stream(res.bytes_stream());
            let response = axum::response::Response::from_parts(parts, body);
            Ok(response)
        }
    }
}

/// Implementation of `IntoRequest` for `axum::extract::Request`.
///
/// This allows converting an `axum::Request` (from server-side extraction)
/// into a `ClientRequest` that can be sent as an HTTP request. The request's
/// headers and body are transferred from the axum request to the client request.
impl IntoRequest for axum::extract::Request {
    fn into_request(
        self,
        mut request: ClientRequest,
    ) -> impl Future<Output = Result<ClientResponse, RequestError>> + 'static {
        async move {
            let (parts, body) = self.into_parts();

            for (key, value) in &parts.headers {
                request = request.header(key, value)?;
            }

            request
                .send_body_stream(
                    body.into_data_stream()
                        .map(|res| res.map_err(|_| StreamingError::Failed)),
                )
                .await
        }
    }
}
