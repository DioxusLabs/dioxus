use crate::FromResponse;
use crate::IntoRequest;

use dioxus_fullstack_core::{RequestError, ServerFnError};
use reqwest::{RequestBuilder, Url};
use std::prelude::rust_2024::Future;

impl FromResponse for axum::response::Response {
    fn from_response(
        res: reqwest::Response,
    ) -> impl Future<Output = Result<Self, ServerFnError>> + Send {
        async move { todo!() }
    }
}

impl IntoRequest for axum::extract::Request {
    fn into_request(
        self,
        request: RequestBuilder,
    ) -> impl Future<Output = Result<reqwest::Response, reqwest::Error>> + Send + 'static {
        async move { todo!() }
    }
}

trait SomeRes {
    fn from_res(res: impl Clienter);
}

trait Clienter: Sized {
    fn do_it();
}
