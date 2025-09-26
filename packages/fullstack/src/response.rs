use std::{any::Any, prelude::rust_2024::Future};

use axum::extract::FromRequest;
use bytes::Bytes;
use dioxus_fullstack_core::DioxusServerState;
use http::HeaderMap;
use reqwest::RequestBuilder;
use serde::{
    de::{DeserializeOwned, DeserializeSeed},
    Deserialize, Serialize,
};

use crate::IntoRequest;

impl IntoRequest for axum::extract::Request {
    fn into_request(
        self,
        request: RequestBuilder,
    ) -> impl Future<Output = Result<reqwest::Response, reqwest::Error>> + Send + 'static {
        async move { todo!() }
    }
}
