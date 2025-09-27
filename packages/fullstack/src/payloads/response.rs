use crate::{ClientRequest, IntoRequest};
use crate::{ClientResponse, FromResponse};

use dioxus_fullstack_core::{RequestError, ServerFnError};
use std::future::Future;

impl FromResponse for axum::response::Response {
    fn from_response(
        res: ClientResponse,
    ) -> impl Future<Output = Result<Self, ServerFnError>> + Send {
        async move { todo!() }
    }
}

impl IntoRequest for axum::extract::Request {
    fn into_request(
        self,
        request: ClientRequest,
    ) -> impl Future<Output = Result<ClientResponse, RequestError>> + Send + 'static {
        async move { todo!() }
    }
}
