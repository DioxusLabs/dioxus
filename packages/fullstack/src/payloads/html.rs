use std::future::Future;

use axum::response::{Html, IntoResponse};
use dioxus_fullstack_core::ServerFnError;
use serde::de::DeserializeOwned;

use crate::{ClientResponse, FromResponse};

impl<T: From<String>> FromResponse for Html<T> {
    fn from_response(res: ClientResponse) -> impl Future<Output = Result<Self, ServerFnError>> {
        async move {
            todo!("Implement Html<T> deserialization")
            // let res = res.text().await?;

            // Ok(Html(res))
        }
    }
}
