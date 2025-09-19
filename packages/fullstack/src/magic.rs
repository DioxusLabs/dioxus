//! ServerFn request magical 🧙 decoders and encoders.
//!
//! The Dioxus Server Function implementation brings a lot of *magic* to the types of endpoints we can handle.
//! Our ultimate goal is to handle *all* endpoints, even axum endpoints, with the macro.
//!
//! Unfortunately, some axum traits like `FromRequest` overlap with some of the default magic we want
//! to provide, like allowing DeserializedOwned groups.
//!
//! Our ultimate goal - to accept all axum handlers - is feasible but not fully implemented.
//!
//! Broadly, we support the following categories of handlers arguments:
//! - Handlers with a single argument that implements `FromRequest` + `IntoRequest`
//! - Handlers with multiple arguments that implement *all* `DeserializeOwned` (and thus can be deserialized from a JSON body)
//!
//! The handler error return types we support are:
//! - Result<T, E> where E: From<ServerFnError> + Serialize + DeserializeOwned (basically any custom `thiserror` impl)
//! - Result<T, anyhow::Error> where we transport the error as a string and/or through ServerFnError
//!
//! The handler return types we support are:
//! - T where T: FromResponse
//! - T where T: DeserializeOwned
//!
//! Note that FromResponse and IntoRequest are *custom* traits defined in this crate. The intention
//! is to provide "inverse" traits of the axum traits, allowing types to flow seamlessly between client and server.
//!
//! These are unfortunately in conflict with the serialization traits. Types like `Bytes` implement both
//! IntoResponse and Serialize, so what should you use?
//!
//! This module implements auto-deref specialization to allow tiering of the above cases.
//!
//! This is sadly quite "magical", but it works. Because the FromResponse traits are defined in this crate,
//! they are sealed against types that implement Deserialize/Serialize, meaning you cannot implement
//! FromResponse for a type that implements Serialize.
//!
//! This module is broken up into several parts, attempting to match how the server macro generates code:
//! - ReqwestEncoder: encodes a set of arguments into a reqwest request

use crate::FromResponse;
use crate::ServerFnRejection;
use crate::{IntoRequest, ServerFnError};
use axum::response::IntoResponse;
use axum::Json;
use axum_core::extract::{FromRequest, Request};
use bytes::Bytes;
use dioxus_fullstack_core::DioxusServerState;
use futures::FutureExt;
use http::HeaderMap;
use send_wrapper::SendWrapper;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::{
    any::{type_name, TypeId},
    marker::PhantomData,
    pin::Pin,
    prelude::rust_2024::Future,
};

type Res = Result<reqwest::Response, reqwest::Error>;

#[doc(hidden)]
pub struct ServerFnEncoder<In, Out>(PhantomData<fn() -> (In, Out)>);
impl<In, Out> ServerFnEncoder<In, Out> {
    #[doc(hidden)]
    pub fn new() -> Self {
        ServerFnEncoder(PhantomData)
    }
}

#[doc(hidden)]
pub struct ServerFnDecoder<Out>(PhantomData<fn() -> Out>);
impl<Out> ServerFnDecoder<Out> {
    #[doc(hidden)]
    pub fn new() -> Self {
        ServerFnDecoder(PhantomData)
    }
}

pub use req_to::*;
pub mod req_to {
    use super::*;

    pub struct FetchRequest {
        pub client: reqwest::RequestBuilder,
    }
    impl FetchRequest {
        pub fn new(method: http::Method, url: String) -> Self {
            let client = reqwest::Client::new();
            let client = client.request(method, url);
            Self { client }
        }
    }

    pub trait EncodeRequest<In, Out> {
        fn fetch_client(
            &self,
            ctx: FetchRequest,
            data: In,
            map: fn(In) -> Out,
        ) -> impl Future<Output = Res> + Send + 'static;
    }

    // One-arg case
    impl<T, O> EncodeRequest<T, O> for &&&&&&&&&&ServerFnEncoder<T, O>
    where
        T: DeserializeOwned + Serialize + 'static,
    {
        fn fetch_client(
            &self,
            ctx: FetchRequest,
            data: T,
            _map: fn(T) -> O,
        ) -> impl Future<Output = Res> + Send + 'static {
            send_wrapper::SendWrapper::new(async move {
                let data = serde_json::to_string(&data).unwrap();

                if data.is_empty() || data == "{}" {
                    return Ok(ctx.client.send().await.unwrap());
                }

                Ok(ctx.client.body(data).send().await.unwrap())
            })
        }
    }

    impl<T, O> EncodeRequest<T, O> for &&&&&&&&&ServerFnEncoder<T, O>
    where
        T: 'static,
        O: FromRequest<DioxusServerState> + IntoRequest,
    {
        fn fetch_client(
            &self,
            ctx: FetchRequest,
            data: T,
            map: fn(T) -> O,
        ) -> impl Future<Output = Res> + Send + 'static {
            O::into_request(map(data), ctx.client)
        }
    }
}

pub use decode_ok::*;
mod decode_ok {
    use super::*;
    use http::StatusCode;

    /// Conver the reqwest response into the desired type, in place.
    /// The point here is to prefer FromResponse types *first* and then DeserializeOwned types second.
    ///
    /// This is because FromResponse types are more specialized and can handle things like websockets and files.
    /// DeserializeOwned types are more general and can handle things like JSON responses.
    pub trait ReqwestDecodeResult<T> {
        fn decode_client_response(
            &self,
            res: Result<reqwest::Response, reqwest::Error>,
        ) -> impl Future<Output = Result<Result<T, ServerFnError>, reqwest::Error>> + Send;
    }

    impl<T: FromResponse, E> ReqwestDecodeResult<T> for &&&ServerFnDecoder<Result<T, E>> {
        fn decode_client_response(
            &self,
            res: Result<reqwest::Response, reqwest::Error>,
        ) -> impl Future<Output = Result<Result<T, ServerFnError>, reqwest::Error>> + Send {
            SendWrapper::new(async move {
                match res {
                    Err(err) => Err(err),
                    Ok(res) => Ok(T::from_response(res).await),
                }
            })
        }
    }

    impl<T: DeserializeOwned, E> ReqwestDecodeResult<T> for &&ServerFnDecoder<Result<T, E>> {
        fn decode_client_response(
            &self,
            res: Result<reqwest::Response, reqwest::Error>,
        ) -> impl Future<Output = Result<Result<T, ServerFnError>, reqwest::Error>> + Send {
            SendWrapper::new(async move {
                match res {
                    Err(err) => Err(err),
                    Ok(res) => {
                        let bytes = res.bytes().await.unwrap();
                        let as_bytes = if bytes.is_empty() {
                            b"null".as_slice()
                        } else {
                            &bytes
                        };
                        let res = serde_json::from_slice::<T>(as_bytes);
                        match res {
                            Ok(t) => Ok(Ok(t)),
                            Err(e) => Ok(Err(ServerFnError::Deserialization(e.to_string()))),
                        }
                    }
                }
            })
        }
    }

    pub trait ReqwestDecodeErr<T, E> {
        fn decode_client_err(
            &self,
            res: Result<Result<T, ServerFnError>, reqwest::Error>,
        ) -> impl Future<Output = Result<T, E>> + Send;
    }

    impl<T, E> ReqwestDecodeErr<T, E> for &&&ServerFnDecoder<Result<T, E>>
    where
        E: From<ServerFnError> + DeserializeOwned + Serialize,
    {
        fn decode_client_err(
            &self,
            res: Result<Result<T, ServerFnError>, reqwest::Error>,
        ) -> impl Future<Output = Result<T, E>> + Send {
            SendWrapper::new(async move {
                match res {
                    Ok(Ok(res)) => Ok(res),
                    Ok(Err(e)) => Err(e.into()),
                    // todo: implement proper through-error conversion, instead of just ServerFnError::Request
                    // we should expand these cases.
                    Err(err) => Err(ServerFnError::Request {
                        message: err.to_string(),
                        code: err.status().map(|s| s.as_u16()),
                    }
                    .into()),
                }
            })
        }
    }

    /// Here we convert to ServerFnError and then into the anyhow::Error, letting the user downcast
    /// from the ServerFnError if they want to.
    ///
    /// This loses any actual type information, but is the most flexible for users.
    impl<T> ReqwestDecodeErr<T, anyhow::Error> for &&ServerFnDecoder<Result<T, anyhow::Error>> {
        fn decode_client_err(
            &self,
            res: Result<Result<T, ServerFnError>, reqwest::Error>,
        ) -> impl Future<Output = Result<T, anyhow::Error>> + Send {
            SendWrapper::new(async move {
                match res {
                    Ok(Ok(res)) => Ok(res),
                    Ok(Err(e)) => Err(anyhow::Error::from(e)),
                    Err(err) => Err(anyhow::Error::from(ServerFnError::Request {
                        message: err.to_string(),
                        code: err.status().map(|s| s.as_u16()),
                    })),
                }
            })
        }
    }

    /// This converts to statuscode, which can be useful but loses a lot of information.
    impl<T> ReqwestDecodeErr<T, StatusCode> for &ServerFnDecoder<Result<T, StatusCode>> {
        fn decode_client_err(
            &self,
            res: Result<Result<T, ServerFnError>, reqwest::Error>,
        ) -> impl Future<Output = Result<T, StatusCode>> + Send {
            SendWrapper::new(async move {
                match res {
                    Ok(Ok(res)) => Ok(res),
                    Ok(Err(e)) => {
                        //
                        match e {
                            // todo: we've caught the reqwest error here, so we should give it back in the form of a proper status code.
                            ServerFnError::Request { message, code } => {
                                Err(StatusCode::INTERNAL_SERVER_ERROR)
                            }

                            ServerFnError::ServerError(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
                            ServerFnError::Registration(_) => {
                                Err(StatusCode::INTERNAL_SERVER_ERROR)
                            }
                            ServerFnError::UnsupportedRequestMethod(_) => {
                                Err(StatusCode::INTERNAL_SERVER_ERROR)
                            }

                            ServerFnError::MiddlewareError(_) => {
                                Err(StatusCode::INTERNAL_SERVER_ERROR)
                            }
                            ServerFnError::Deserialization(_) => {
                                Err(StatusCode::INTERNAL_SERVER_ERROR)
                            }
                            ServerFnError::Serialization(_) => {
                                Err(StatusCode::INTERNAL_SERVER_ERROR)
                            }
                            ServerFnError::Args(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
                            ServerFnError::MissingArg(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
                            ServerFnError::Response(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
                        }
                    }
                    Err(_) => todo!(),
                }
            })
        }
    }

    /// This tries to catch http::Error and its subtypes, but will not catch everything that is normally "IntoResponse"
    impl<T, E> ReqwestDecodeErr<T, E> for ServerFnDecoder<Result<T, E>>
    where
        E: Into<http::Error>,
        E: From<http::Error>,
    {
        fn decode_client_err(
            &self,
            res: Result<Result<T, ServerFnError>, reqwest::Error>,
        ) -> impl Future<Output = Result<T, E>> + Send {
            SendWrapper::new(async move {
                match res {
                    Ok(Ok(res)) => Ok(res),
                    Ok(Err(e)) => todo!(),
                    Err(_) => todo!(),
                }
            })
        }
    }
}

pub use req_from::*;
pub mod req_from {
    use super::*;

    pub trait ExtractRequest<In, Out> {
        fn extract_axum(
            &self,
            state: DioxusServerState,
            request: Request,
            map: fn(In) -> Out,
        ) -> impl Future<Output = Result<Out, ServerFnRejection>> + Send + 'static;
    }

    // One-arg case
    impl<In, Out> ExtractRequest<In, Out> for &&&&&&&&&&ServerFnEncoder<In, Out>
    where
        In: DeserializeOwned + 'static,
        Out: 'static,
    {
        fn extract_axum(
            &self,
            _state: DioxusServerState,
            request: Request,
            map: fn(In) -> Out,
        ) -> impl Future<Output = Result<Out, ServerFnRejection>> + Send + 'static {
            send_wrapper::SendWrapper::new(async move {
                let bytes = Bytes::from_request(request, &()).await.unwrap();
                let as_str = String::from_utf8_lossy(&bytes);
                tracing::info!("deserializing request body: {}", as_str);
                let bytes = if as_str.is_empty() {
                    "{}".as_bytes()
                } else {
                    &bytes
                };

                serde_json::from_slice::<In>(bytes)
                    .map(map)
                    .map_err(|e| ServerFnRejection {})
            })
        }
    }

    /// We skip the BodySerialize wrapper and just go for the output type directly.
    impl<In, Out> ExtractRequest<In, Out> for &&&&&&&&&ServerFnEncoder<In, Out>
    where
        Out: FromRequest<DioxusServerState> + 'static,
    {
        fn extract_axum(
            &self,
            state: DioxusServerState,
            request: Request,
            _map: fn(In) -> Out,
        ) -> impl Future<Output = Result<Out, ServerFnRejection>> + Send + 'static {
            send_wrapper::SendWrapper::new(async move {
                Out::from_request(request, &state)
                    .await
                    .map_err(|e| ServerFnRejection {})
            })
        }
    }
}

pub use resp::*;
mod resp {
    use super::*;

    /// A trait for converting the result of the Server Function into an Axum response.
    ///
    /// This is to work around the issue where we want to return both Deserialize types and FromResponse types.
    /// Stuff like websockets
    ///
    /// We currently have an `Input` type even though it's not useful since we might want to support regular axum endpoints later.
    /// For now, it's just Result<T, E> where T is either DeserializeOwned or FromResponse
    pub trait FromResIt<T> {
        fn make_axum_response(self, result: T) -> axum::response::Response;
    }

    // Higher priority impl for special types like websocket/file responses that generate their own responses
    // The FromResponse impl helps narrow types to those usable on the client
    impl<T, E> FromResIt<Result<T, E>> for &&&ServerFnDecoder<Result<T, E>>
    where
        T: FromResponse + IntoResponse,
        E: From<ServerFnError>,
    {
        fn make_axum_response(self, result: Result<T, E>) -> axum::response::Response {
            match result {
                Ok(res) => res.into_response(),
                Err(err) => todo!(),
            }
        }
    }

    // Lower priority impl for regular serializable types
    // We try to match the encoding from the incoming request, otherwise default to JSON
    impl<T, E> FromResIt<Result<T, E>> for &&ServerFnDecoder<Result<T, E>>
    where
        T: DeserializeOwned + Serialize,
        E: From<ServerFnError>,
    {
        fn make_axum_response(self, result: Result<T, E>) -> axum::response::Response {
            match result.map(|v| serde_json::to_string(&v)) {
                Ok(Ok(v)) => {
                    let mut res = (axum::http::StatusCode::OK, v).into_response();
                    res.headers_mut().insert(
                        axum::http::header::CONTENT_TYPE,
                        axum::http::HeaderValue::from_static("application/json"),
                    );
                    res
                }
                Ok(Err(e)) => {
                    todo!()
                }
                Err(e) => {
                    todo!()
                }
            }
        }
    }
}
