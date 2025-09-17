use axum_core::response::{IntoResponse, Response};
use dioxus_fullstack_core::ServerFnError;
use send_wrapper::SendWrapper;
use std::prelude::rust_2024::Future;

use crate::FromResponse;

pub struct Text<T>(pub T);

impl<T: Into<String>> IntoResponse for Text<T> {
    fn into_response(self) -> Response {
        Response::builder()
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(axum_core::body::Body::from(self.0.into()))
            .unwrap()
    }
}

impl<T: Into<String>> FromResponse for Text<T> {
    fn from_response(
        res: reqwest::Response,
    ) -> impl Future<Output = Result<Self, ServerFnError>> + Send {
        SendWrapper::new(async move { todo!() })
    }
}
