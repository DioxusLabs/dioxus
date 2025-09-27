use crate::{ClientResponse, FromResponse};
use axum::response::Html;
use dioxus_fullstack_core::ServerFnError;
use std::future::Future;

impl<T: From<String>> FromResponse for Html<T> {
    fn from_response(res: ClientResponse) -> impl Future<Output = Result<Self, ServerFnError>> {
        async move {
            let content = res.text().await?;
            Ok(Html(content.into()))
        }
    }
}
