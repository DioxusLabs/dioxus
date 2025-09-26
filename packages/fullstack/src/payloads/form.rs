use std::prelude::rust_2024::Future;

pub use axum::Form;
use dioxus_fullstack_core::RequestError;

use crate::{ClientResponse, IntoRequest};

impl<T> IntoRequest for Form<T> {
    fn into_request(
        self,
        builder: reqwest::RequestBuilder,
    ) -> impl Future<Output = Result<ClientResponse, RequestError>> + Send + 'static {
        async move { todo!() }
    }
}

//     const CONTENT_TYPE: &'static str = "application/x-www-form-urlencoded";
//     const CONTENT_TYPE: &'static str = "application/x-www-form-urlencoded";
// //     const CONTENT_TYPE: &'static str = "application/x-www-form-urlencoded";
// //     const CONTENT_TYPE: &'static str = "application/x-www-form-urlencoded";
// //     const CONTENT_TYPE: &'static str = "application/x-www-form-urlencoded";
