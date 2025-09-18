use std::prelude::rust_2024::Future;

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
    headers: HeaderMap,
    status: http::StatusCode,
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
        input: Self,
        request_builder: reqwest::RequestBuilder,
    ) -> impl Future<Output = Result<reqwest::Response, reqwest::Error>> + Send + 'static {
        async move { todo!() }
    }
}
