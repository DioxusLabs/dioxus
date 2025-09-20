use axum::{
    extract::{FromRequest, Request},
    response::IntoResponse,
};
use dioxus_fullstack_core::DioxusServerState;
use std::future::Future;

pub struct Jwt {}

impl IntoResponse for Jwt {
    fn into_response(self) -> axum::response::Response {
        todo!()
    }
}

impl<S> FromRequest<S> for Jwt {
    type Rejection = axum::response::Response;

    fn from_request(
        req: Request,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move { todo!() }
    }
}
