use super::*;
use crate::{ClientResponse, FromResponse};
pub use axum::extract::Json;
use axum::response::{Html, NoContent, Redirect};
use dioxus_fullstack_core::{RequestError, ServerFnError};
use futures::StreamExt;
use http::StatusCode;
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
    T: Serialize + 'static + DeserializeOwned,
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

impl FromResponse for Redirect {
    fn from_response(res: ClientResponse) -> impl Future<Output = Result<Self, ServerFnError>> {
        async move {
            let location = res
                .headers()
                .get(http::header::LOCATION)
                .ok_or_else(|| RequestError::Redirect("Missing Location header".into()))?
                .to_str()
                .map_err(|_| RequestError::Redirect("Invalid Location header".into()))?;
            match res.status() {
                StatusCode::SEE_OTHER => Ok(Redirect::to(location)),
                StatusCode::TEMPORARY_REDIRECT => Ok(Redirect::temporary(location)),
                StatusCode::PERMANENT_REDIRECT => Ok(Redirect::permanent(location)),
                _ => Err(RequestError::Redirect("Not a redirect status code".into()).into()),
            }
        }
    }
}

impl FromResponse for NoContent {
    fn from_response(res: ClientResponse) -> impl Future<Output = Result<Self, ServerFnError>> {
        async move {
            let status = res.status();
            if status == StatusCode::NO_CONTENT {
                Ok(NoContent)
            } else {
                let body = res.text().await.unwrap_or_else(|_| "".into());
                Err(RequestError::Status(body, status.into()).into())
            }
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
