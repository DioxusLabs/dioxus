use dioxus_fullstack_core::{RequestError, ServerFnError};
use reqwest::RequestBuilder;
use std::prelude::rust_2024::Future;

use crate::IntoRequest;

impl IntoRequest for axum::extract::Request {
    fn into_request(
        self,
        request: RequestBuilder,
    ) -> impl Future<Output = Result<reqwest::Response, reqwest::Error>> + Send + 'static {
        async move { todo!() }
    }
}

/// Convert a `reqwest::Error` into a `ServerFnError`.
pub fn reqwest_resonse_to_serverfn_err(err: reqwest::Error) -> ServerFnError {
    let inner = if err.is_timeout() {
        RequestError::Timeout
    } else if err.is_connect() {
        RequestError::Connect
    } else if err.is_request() {
        RequestError::Request
    } else if err.is_body() {
        RequestError::Body
    } else if err.is_decode() {
        RequestError::Decode
    } else if err.is_redirect() {
        RequestError::Redirect
    } else if let Some(status) = err.status() {
        RequestError::Status(status.as_u16())
    } else {
        RequestError::Request
    };

    ServerFnError::Request {
        error: inner,
        message: err.to_string(),
    }
}
