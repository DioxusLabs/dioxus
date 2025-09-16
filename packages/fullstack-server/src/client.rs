use crate::ServerFnError;
use axum::{extract::Request, response::Response};
use axum::{
    extract::{FromRequest, FromRequestParts},
    response::IntoResponse,
    Json,
};
use bytes::Bytes;
use dioxus_fullstack_core::ServerFnSugar;
use futures::Stream;
use http::{request::Parts, Error, Method};
use http_body_util::BodyExt;
use reqwest::RequestBuilder;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::sync::OnceLock;
use std::{future::Future, str::FromStr, sync::LazyLock};

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| reqwest::Client::new());

pub async fn make_request<R: SharedClientType<M>, M>(
    method: Method,
    url: &str,
    params: impl Serialize,
) -> Result<R::Output, ServerFnError> {
    let res = CLIENT.request(method, url).query(&params).send().await;
    let res = res.unwrap();
    let res = R::decode(&CLIENT, res).await;
    res
}

/// A trait representing a type that can be used as the return type of a server function on the client side.
/// This trait is implemented for types that can be deserialized from the response of a server function.
/// The default encoding is JSON, but this can be customized by wrapping the output type in a newtype
/// that implements this trait.
///
/// A number of common wrappers are provided, such as `axum::Json<T>`, which will decode the response.
/// We provide other types like Cbor/MessagePack for different encodings.
pub trait SharedClientType<M = ()> {
    type Output;
    fn encode(item: &Self::Output) {}
    fn decode(
        client: &reqwest::Client,
        res: reqwest::Response,
    ) -> impl Future<Output = Result<Self::Output, ServerFnError>> + Send;
}

/// Use the default encoding, which is usually json but can be configured to be something else
pub struct DefaultEncodeMarker;
impl<T: DeserializeOwned> SharedClientType<DefaultEncodeMarker> for T {
    type Output = T;
    async fn decode(
        client: &reqwest::Client,
        res: reqwest::Response,
    ) -> Result<Self, ServerFnError> {
        let bytes = res.bytes().await.unwrap();
        let res = serde_json::from_slice(&bytes).unwrap();
        Ok(res)
    }
}

impl<T: DeserializeOwned> SharedClientType for Json<T> {
    type Output = Json<T>;
    async fn decode(
        client: &reqwest::Client,
        res: reqwest::Response,
    ) -> Result<Self, ServerFnError> {
        let bytes = res.bytes().await.unwrap();
        let res = serde_json::from_slice(&bytes).unwrap();
        Ok(Json(res))
    }
}
