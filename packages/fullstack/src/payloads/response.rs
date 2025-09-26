use std::prelude::rust_2024::Future;

use dioxus_fullstack_core::ServerFnError;

use crate::FromResponse;

impl FromResponse for axum::response::Response {
    fn from_response(
        res: reqwest::Response,
    ) -> impl Future<Output = Result<Self, ServerFnError>> + Send {
        async move { todo!() }
    }
}
