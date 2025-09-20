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
use axum_core::extract::{FromRequest, Request};
use bytes::Bytes;
use dioxus_fullstack_core::DioxusServerState;
use http::StatusCode;
use send_wrapper::SendWrapper;
use serde::Serialize;
use serde::{de::DeserializeOwned, Deserialize};
use std::fmt::Display;
use std::{marker::PhantomData, prelude::rust_2024::Future};

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

/// A response structure for a regular REST API, with a success and error case where the status is
/// encoded in the body and all fields are serializable. This lets you call fetch().await.json()
/// and get a strongly typed result.
///
/// Eventually we want to support JsonRPC which requires a different format.
///
/// We use the `___status` field to avoid conflicts with user-defined fields. Hopefully no one uses this field name!
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum RestEndpointPayload<T, E> {
    #[serde(rename = "success")]
    Success(T),

    #[serde(rename = "error")]
    Error(ErrorPayload<E>),
}

/// The error payload structure for REST API errors.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ErrorPayload<E> {
    message: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<u16>,

    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<E>,
}

pub use req_to::*;
pub mod req_to {
    use crate::{CantEncode, EncodeIsVerified};

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
        type VerifyEncode;
        fn fetch_client(
            &self,
            ctx: FetchRequest,
            data: In,
            map: fn(In) -> Out,
        ) -> impl Future<Output = Result<reqwest::Response, reqwest::Error>> + Send + 'static;

        fn verify_can_serialize(&self) -> Self::VerifyEncode;
    }

    // One-arg case
    impl<T, O> EncodeRequest<T, O> for &&&&&&&&&&ServerFnEncoder<T, O>
    where
        T: DeserializeOwned + Serialize + 'static,
    {
        type VerifyEncode = EncodeIsVerified;
        fn fetch_client(
            &self,
            ctx: FetchRequest,
            data: T,
            _map: fn(T) -> O,
        ) -> impl Future<Output = Result<reqwest::Response, reqwest::Error>> + Send + 'static
        {
            send_wrapper::SendWrapper::new(async move {
                let data = serde_json::to_string(&data).unwrap();

                if data.is_empty() || data == "{}" {
                    return Ok(ctx.client.send().await.unwrap());
                }

                Ok(ctx.client.body(data).send().await.unwrap())
            })
        }

        fn verify_can_serialize(&self) -> Self::VerifyEncode {
            EncodeIsVerified
        }
    }

    impl<T, O> EncodeRequest<T, O> for &&&&&&&&&ServerFnEncoder<T, O>
    where
        T: 'static,
        O: FromRequest<DioxusServerState> + IntoRequest,
    {
        type VerifyEncode = EncodeIsVerified;
        fn fetch_client(
            &self,
            ctx: FetchRequest,
            data: T,
            map: fn(T) -> O,
        ) -> impl Future<Output = Result<reqwest::Response, reqwest::Error>> + Send + 'static
        {
            O::into_request(map(data), ctx.client)
        }

        fn verify_can_serialize(&self) -> Self::VerifyEncode {
            EncodeIsVerified
        }
    }

    /// The fall-through case that emits a `CantEncode` type which fails to compile when checked by the macro
    impl<T, O> EncodeRequest<T, O> for &ServerFnEncoder<T, O>
    where
        T: 'static,
    {
        type VerifyEncode = CantEncode;
        #[allow(clippy::manual_async_fn)]
        fn fetch_client(
            &self,
            _ctx: FetchRequest,
            _data: T,
            _map: fn(T) -> O,
        ) -> impl Future<Output = Result<reqwest::Response, reqwest::Error>> + Send + 'static
        {
            async move { unimplemented!() }
        }
        fn verify_can_serialize(&self) -> Self::VerifyEncode {
            CantEncode
        }
    }
}

pub use decode_ok::*;
mod decode_ok {
    use super::*;

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
                        let status = res.status();

                        let bytes = res.bytes().await.unwrap();
                        let as_bytes = if bytes.is_empty() {
                            b"null".as_slice()
                        } else {
                            &bytes
                        };

                        let res = if status.is_success() {
                            serde_json::from_slice::<T>(as_bytes).map(RestEndpointPayload::Success)
                        } else {
                            serde_json::from_slice::<ErrorPayload<serde_json::Value>>(as_bytes)
                                .map(RestEndpointPayload::Error)
                        };

                        match res {
                            Ok(RestEndpointPayload::Success(t)) => Ok(Ok(t)),
                            Ok(RestEndpointPayload::Error(err)) => {
                                Ok(Err(ServerFnError::ServerError {
                                    message: err.message,
                                    details: err.data,
                                    code: err.code,
                                }))
                            }
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
                    Ok(Err(e)) => match e {
                        ServerFnError::ServerError {
                            details,
                            message,
                            code,
                        } => {
                            // If there are "details", then we try to deserialize them into the error type.
                            // If there aren't, we just create a generic ServerFnError::ServerError with the message.
                            match details {
                                Some(details) => match serde_json::from_value::<E>(details) {
                                    Ok(res) => Err(res),
                                    Err(err) => Err(E::from(ServerFnError::Deserialization(
                                        err.to_string(),
                                    ))),
                                },
                                None => Err(E::from(ServerFnError::ServerError {
                                    message,
                                    details: None,
                                    code,
                                })),
                            }
                        }
                        err => Err(err.into()),
                    },
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

                    // We do a best-effort conversion from ServerFnError to StatusCode.
                    Ok(Err(e)) => match e {
                        ServerFnError::Request {
                            message: _message,
                            code,
                        } => Err(StatusCode::from_u16(code.unwrap_or(500))
                            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)),

                        ServerFnError::ServerError {
                            message: _message,
                            details: _details,
                            code,
                        } => Err(StatusCode::from_u16(code.unwrap_or(500))
                            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)),

                        ServerFnError::Registration(_) | ServerFnError::MiddlewareError(_) => {
                            Err(StatusCode::INTERNAL_SERVER_ERROR)
                        }

                        ServerFnError::Deserialization(_)
                        | ServerFnError::Serialization(_)
                        | ServerFnError::Args(_)
                        | ServerFnError::MissingArg(_) => Err(StatusCode::UNPROCESSABLE_ENTITY),

                        ServerFnError::UnsupportedRequestMethod(_) => {
                            Err(StatusCode::METHOD_NOT_ALLOWED)
                        }

                        ServerFnError::Response(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
                    },

                    // The reqwest error case, we try to convert the reqwest error into a status code.
                    Err(reqwest_err) => {
                        let code = reqwest_err
                            .status()
                            .unwrap_or(StatusCode::SERVICE_UNAVAILABLE);
                        Err(code)
                    }
                }
            })
        }
    }
}

pub use req_from::*;
pub mod req_from {
    use super::*;
    use axum::{extract::FromRequestParts, response::Response};

    pub trait ExtractRequest<In, Out, M = ()> {
        fn extract_axum(
            &self,
            state: DioxusServerState,
            request: Request,
            map: fn(In) -> Out,
        ) -> impl Future<Output = Result<Out, Response>> + Send + 'static;
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
        ) -> impl Future<Output = Result<Out, Response>> + Send + 'static {
            send_wrapper::SendWrapper::new(async move {
                let bytes = Bytes::from_request(request, &()).await.unwrap();
                let as_str = String::from_utf8_lossy(&bytes);

                let bytes = if as_str.is_empty() {
                    "{}".as_bytes()
                } else {
                    &bytes
                };

                serde_json::from_slice::<In>(bytes)
                    .map(map)
                    .map_err(|e| ServerFnRejection {}.into_response())
            })
        }
    }

    /// We skip the BodySerialize wrapper and just go for the output type directly.
    impl<In, Out, M> ExtractRequest<In, Out, M> for &&&&&&&&&ServerFnEncoder<In, Out>
    where
        Out: FromRequest<DioxusServerState, M> + 'static,
    {
        fn extract_axum(
            &self,
            state: DioxusServerState,
            request: Request,
            _map: fn(In) -> Out,
        ) -> impl Future<Output = Result<Out, Response>> + Send + 'static {
            send_wrapper::SendWrapper::new(async move {
                Out::from_request(request, &state)
                    .await
                    .map_err(|e| ServerFnRejection {}.into_response())
            })
        }
    }
}

pub use resp::*;
mod resp {
    use super::*;
    use axum::response::Response;
    use http::HeaderValue;

    /// A trait for converting the result of the Server Function into an Axum response.
    ///
    /// This is to work around the issue where we want to return both Deserialize types and FromResponse types.
    /// Stuff like websockets
    ///
    /// We currently have an `Input` type even though it's not useful since we might want to support regular axum endpoints later.
    /// For now, it's just Result<T, E> where T is either DeserializeOwned or FromResponse
    pub trait MakeAxumResponse<T, E> {
        fn make_axum_response(self, result: Result<T, E>) -> Result<Response, E>;
    }

    // Higher priority impl for special types like websocket/file responses that generate their own responses
    // The FromResponse impl helps narrow types to those usable on the client
    impl<T, E> MakeAxumResponse<T, E> for &&&ServerFnDecoder<Result<T, E>>
    where
        T: FromResponse + IntoResponse,
    {
        fn make_axum_response(self, result: Result<T, E>) -> Result<Response, E> {
            result.map(|v| v.into_response())
        }
    }

    // Lower priority impl for regular serializable types
    // We try to match the encoding from the incoming request, otherwise default to JSON
    impl<T, E> MakeAxumResponse<T, E> for &&ServerFnDecoder<Result<T, E>>
    where
        T: DeserializeOwned + Serialize,
    {
        fn make_axum_response(self, result: Result<T, E>) -> Result<Response, E> {
            match result {
                Ok(res) => {
                    let body = serde_json::to_string(&res).unwrap();
                    let mut resp = Response::new(body.into());
                    resp.headers_mut().insert(
                        http::header::CONTENT_TYPE,
                        HeaderValue::from_static("application/json"),
                    );
                    *resp.status_mut() = StatusCode::OK;
                    Ok(resp)
                }
                Err(err) => Err(err),
            }
        }
    }

    #[allow(clippy::result_large_err)]
    pub trait MakeAxumError<E> {
        fn make_axum_error(self, result: Result<Response, E>) -> Result<Response, Response>;
    }

    impl<T, E> MakeAxumError<E> for &&&ServerFnDecoder<Result<T, E>>
    where
        E: From<ServerFnError> + Serialize + DeserializeOwned + Display,
    {
        fn make_axum_error(self, result: Result<Response, E>) -> Result<Response, Response> {
            match result {
                Ok(res) => Ok(res),
                Err(err) => {
                    let err = ErrorPayload {
                        code: None,
                        message: err.to_string(),
                        data: Some(err),
                    };
                    let body = serde_json::to_string(&err).unwrap();
                    let mut resp = Response::new(body.into());
                    resp.headers_mut().insert(
                        http::header::CONTENT_TYPE,
                        HeaderValue::from_static("application/json"),
                    );
                    *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                    Err(resp)
                }
            }
        }
    }

    impl<T> MakeAxumError<anyhow::Error> for &&ServerFnDecoder<Result<T, anyhow::Error>> {
        fn make_axum_error(
            self,
            result: Result<Response, anyhow::Error>,
        ) -> Result<Response, Response> {
            match result {
                Ok(res) => Ok(res),
                Err(errr) => {
                    // The `WithHttpError` trait emits ServerFnErrors so we can downcast them here
                    // to create richer responses.
                    let payload = match errr.downcast::<ServerFnError>() {
                        Ok(ServerFnError::ServerError {
                            message,
                            code,
                            details,
                        }) => ErrorPayload {
                            message,
                            code,
                            data: details,
                        },
                        Ok(other) => ErrorPayload {
                            message: other.to_string(),
                            code: None,
                            data: None,
                        },
                        Err(err) => ErrorPayload {
                            code: None,
                            message: err.to_string(),
                            data: None,
                        },
                    };

                    let body = serde_json::to_string(&payload).unwrap();
                    let mut resp = Response::new(body.into());
                    resp.headers_mut().insert(
                        http::header::CONTENT_TYPE,
                        HeaderValue::from_static("application/json"),
                    );
                    *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                    Err(resp)
                }
            }
        }
    }

    impl<T> MakeAxumError<StatusCode> for &&ServerFnDecoder<Result<T, StatusCode>> {
        fn make_axum_error(
            self,
            result: Result<Response, StatusCode>,
        ) -> Result<Response, Response> {
            match result {
                Ok(resp) => Ok(resp),
                Err(status) => {
                    let body = serde_json::to_string(&ErrorPayload::<()> {
                        code: Some(status.as_u16()),
                        message: status.to_string(),
                        data: None,
                    })
                    .unwrap();
                    let mut resp = Response::new(body.into());
                    resp.headers_mut().insert(
                        http::header::CONTENT_TYPE,
                        HeaderValue::from_static("application/json"),
                    );
                    *resp.status_mut() = status;
                    Err(resp)
                }
            }
        }
    }
}
