use std::{any::Any, prelude::rust_2024::Future};

use axum::extract::FromRequest;
use bytes::Bytes;
use dioxus_fullstack_core::DioxusServerState;
use http::HeaderMap;
use serde::{
    de::{DeserializeOwned, DeserializeSeed},
    Deserialize, Serialize,
};

use crate::IntoRequest;

pub struct ServerResponse {
    inner: Box<dyn PlatformResponse>,
}

impl ServerResponse {
    pub async fn new_from_reqwest(res: reqwest::Response) -> Self {
        let status = res.status();
        let headers = res.headers();
        todo!()
    }
}

impl IntoRequest for axum::extract::Request {
    fn into_request(
        self,
        request_builder: reqwest::RequestBuilder,
    ) -> impl Future<Output = Result<reqwest::Response, reqwest::Error>> + Send + 'static {
        async move { todo!() }
    }
}

/// A response that wraps a `reqwest::Response` and optionally holds some state that can be used
/// across sending and receiving the response.
///
/// Useful for things like websockets that need to hold onto the upgrade state across the request/response boundary.
pub struct ResponseWithState {
    pub response: reqwest::Response,
    pub state: Option<Box<dyn Any>>,
}

pub trait PlatformResponse {}
