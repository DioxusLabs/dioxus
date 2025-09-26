use crate::FromResponse;
use axum_core::response::{IntoResponse, Response};
use dioxus_fullstack_core::ServerFnError;
use send_wrapper::SendWrapper;
use std::prelude::rust_2024::Future;

/// A simple text response type.
///
/// The `T` parameter can be anything that converts to and from `String`, such as `Rc<str>` or `String`.
///
/// Unlike `Json` or plain `String`, this uses the `text/plain` content type. The `text/plain` header
/// will be set on the request.
pub struct Text<T>(pub T);

impl<T> Text<T> {
    /// Create a new text response.
    pub fn new(text: T) -> Self {
        Self(text)
    }
}

impl<T: Into<String>> IntoResponse for Text<T> {
    fn into_response(self) -> Response {
        Response::builder()
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(axum_core::body::Body::from(self.0.into()))
            .unwrap()
    }
}

impl<T: From<String>> FromResponse for Text<T> {
    fn from_response(
        res: reqwest::Response,
    ) -> impl Future<Output = Result<Self, ServerFnError>> + Send {
        SendWrapper::new(async move {
            match res.text().await {
                Ok(text) => Ok(Text::new(text.into())),
                Err(err) => Err(todo!("handle error: {}", err)),
            }
        })
    }
}
