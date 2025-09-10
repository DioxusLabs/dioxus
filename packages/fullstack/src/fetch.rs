use crate::{Decodes, Encodes, ServerFnError};
use axum::{
    extract::{FromRequest, FromRequestParts},
    response::IntoResponse,
    Json,
};
use bytes::Bytes;
use futures::Stream;
use http::{request::Parts, Method};
use reqwest::RequestBuilder;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{future::Future, str::FromStr, sync::LazyLock};

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| reqwest::Client::new());

pub fn fetch(method: Method, url: &str) -> RequestBuilder {
    todo!()
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

    // fn decode_stream(
    //     res: reqwest::Response,
    // ) -> impl Stream<Item = Result<Self::Output, ServerFnError>> + Send;
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

pub struct FileUpload {
    outgoing_stream: Option<Box<dyn Stream<Item = Result<Bytes, Bytes>> + Send + Unpin>>,
}

impl FileUpload {
    pub fn from_stream(
        filename: String,
        content_type: String,
        data: impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static,
    ) -> Self {
        todo!()
    }
}
pub struct ServerFnRejection {}
impl IntoResponse for ServerFnRejection {
    fn into_response(self) -> axum::response::Response {
        todo!()
    }
}

impl<S> FromRequest<S> for FileUpload {
    type Rejection = ServerFnRejection;
    fn from_request(
        req: axum::extract::Request,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move { todo!() }
    }
}
// impl<S> FromRequestParts<S> for FileUpload {
//     type Rejection = ServerFnError;

//     fn from_request_parts(
//         parts: &mut Parts,
//         state: &S,
//     ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
//         todo!()
//     }
// }

pub struct FileDownload {}

/// A WebSocket connection that can send and receive messages of type `In` and `Out`.
pub struct WebSocket<In, Out> {
    _in: std::marker::PhantomData<In>,
    _out: std::marker::PhantomData<Out>,
}
impl<In: Serialize, Out: DeserializeOwned> WebSocket<In, Out> {
    pub async fn send(&self, msg: In) -> Result<(), ServerFnError> {
        todo!()
    }

    pub async fn recv(&mut self) -> Result<Out, ServerFnError> {
        todo!()
    }
}

// Create a new WebSocket connection that uses the provided function to handle incoming messages
impl<In, Out> IntoResponse for WebSocket<In, Out> {
    fn into_response(self) -> axum::response::Response {
        todo!()
    }
}

pub trait ServerFnSugar<M> {
    fn to_response(self) -> axum::response::Response;
    fn from_reqwest(res: reqwest::Response) -> Self
    where
        Self: Sized,
    {
        todo!()
    }
}

/// We allow certain error types to be used across both the client and server side
pub trait ErrorSugar {}
impl ErrorSugar for anyhow::Error {}
impl ErrorSugar for ServerFnError {}
impl ErrorSugar for http::Error {}

impl<T> ServerFnSugar<()> for T
where
    T: IntoResponse,
{
    fn to_response(self) -> axum::response::Response {
        todo!()
    }
}

// pub struct DefaultSugarMarkerNoError;
// impl<T: Serialize> ServerFnSugar<DefaultSugarMarker> for T {
//     fn to_response(self) -> axum::response::Response {
//         todo!()
//     }
// }

pub struct DefaultSugarMarker;
impl<T: Serialize, E: IntoResponse> ServerFnSugar<DefaultSugarMarker> for Result<T, E> {
    fn to_response(self) -> axum::response::Response {
        todo!()
    }
}

pub struct SerializeSugarWithErrorMarker;
impl<T: Serialize, E: ErrorSugar> ServerFnSugar<SerializeSugarWithErrorMarker> for Result<T, E> {
    fn to_response(self) -> axum::response::Response {
        todo!()
    }
}

pub struct SerializeSugarMarker;
impl<T: IntoResponse, E: ErrorSugar> ServerFnSugar<SerializeSugarMarker> for Result<T, E> {
    fn to_response(self) -> axum::response::Response {
        todo!()
    }
}

pub fn serverfn_sugar<M>(t: impl ServerFnSugar<M>) -> axum::response::Response {
    t.to_response()
}
