use std::prelude::rust_2024::Future;

pub use axum::Form;

use crate::IntoRequest;

impl<T> IntoRequest for Form<T> {
    fn into_request(
        input: Self,
        builder: reqwest::RequestBuilder,
    ) -> impl Future<Output = Result<reqwest::Response, reqwest::Error>> + Send + 'static {
        async move { todo!() }
    }
}
