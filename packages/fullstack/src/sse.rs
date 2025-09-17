use std::future::Future;

use axum_core::response::IntoResponse;
use axum_core::response::Response;

use crate::{FromResponse, ServerFnError};

pub struct ServerSentEvents<T> {
    _t: std::marker::PhantomData<*const T>,
}

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SseError {}

impl<T> ServerSentEvents<T> {
    pub fn new() -> Self {
        Self {
            _t: std::marker::PhantomData,
        }
    }

    pub async fn next(&mut self) -> Option<Result<T, SseError>> {
        todo!()
    }
}

impl IntoResponse for ServerSentEvents<String> {
    fn into_response(self) -> axum_core::response::Response {
        todo!()
    }
}

impl<T> FromResponse for ServerSentEvents<T> {
    fn from_response(res: Response) -> impl Future<Output = Result<Self, ServerFnError>> + Send {
        async move { Ok(ServerSentEvents::new()) }
    }
}
