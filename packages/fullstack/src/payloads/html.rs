use std::prelude::rust_2024::Future;

use axum::response::Html;
use dioxus_fullstack_core::ServerFnError;
use serde::de::DeserializeOwned;

use crate::{ClientResponse, FromResponse};

impl<T: DeserializeOwned> FromResponse for Html<T> {
    fn from_response(
        res: ClientResponse,
    ) -> impl Future<Output = Result<Self, ServerFnError>> + Send {
        async move { todo!() }
    }
}
