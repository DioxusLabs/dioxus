use crate::{error::ServerFnSugar, FileUpload, ServerFnError};
use axum::{extract::Request, response::Response};
use axum::{
    extract::{FromRequest, FromRequestParts},
    response::IntoResponse,
    Json,
};
use bytes::Bytes;
use futures::Stream;
use http::{request::Parts, Error, Method};
use http_body_util::BodyExt;
use reqwest::RequestBuilder;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::sync::OnceLock;
use std::{future::Future, str::FromStr, sync::LazyLock};

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| reqwest::Client::new());
static ROOT_URL: OnceLock<&'static str> = OnceLock::new();

/// Set the root server URL that all server function paths are relative to for the client.
///
/// If this is not set, it defaults to the origin.
pub fn set_server_url(url: &'static str) {
    ROOT_URL.set(url).unwrap();
}

/// Returns the root server URL for all server functions.
pub fn get_server_url() -> &'static str {
    ROOT_URL.get().copied().unwrap_or("")
}

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

/// We allow certain error types to be used across both the client and server side
/// These need to be able to serialize through the network and end up as a response.
/// Note that the types need to line up, not necessarily be equal.
pub trait ErrorSugar {
    fn to_encode_response(&self) -> Response;
}

impl ErrorSugar for http::Error {
    fn to_encode_response(&self) -> Response {
        todo!()
    }
}

impl<T: From<ServerFnError>> ErrorSugar for T {
    fn to_encode_response(&self) -> Response {
        todo!()
    }
}

/// The default conversion of T into a response is to use axum's IntoResponse trait
/// Note that Result<T: IntoResponse, E: IntoResponse> works as a blanket impl.
pub struct NoSugarMarker;
impl<T: IntoResponse> ServerFnSugar<NoSugarMarker> for T {
    fn desugar_into_response(self) -> Response {
        self.into_response()
    }
}

pub struct SerializeSugarMarker;
impl<T: IntoResponse, E: ErrorSugar> ServerFnSugar<SerializeSugarMarker> for Result<T, E> {
    fn desugar_into_response(self) -> Response {
        todo!()
    }
}

/// This covers the simple case of returning a body from an endpoint where the body is serializable.
/// By default, we use the JSON encoding, but you can use one of the other newtypes to change the encoding.
pub struct DefaultJsonEncodingMarker;
impl<T: Serialize, E: IntoResponse> ServerFnSugar<DefaultJsonEncodingMarker> for &Result<T, E> {
    fn desugar_into_response(self) -> Response {
        todo!()
    }
}

pub struct SerializeSugarWithErrorMarker;
impl<T: Serialize, E: ErrorSugar> ServerFnSugar<SerializeSugarWithErrorMarker> for &Result<T, E> {
    fn desugar_into_response(self) -> Response {
        todo!()
    }
}

/// A newtype wrapper that indicates that the inner type should be converted to a response using its
/// IntoResponse impl and not its Serialize impl.
pub struct ViaResponse<T>(pub T);
impl<T: IntoResponse> IntoResponse for ViaResponse<T> {
    fn into_response(self) -> Response {
        self.0.into_response()
    }
}
