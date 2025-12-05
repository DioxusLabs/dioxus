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
//! - `Result<T, E> where E: From<ServerFnError> + Serialize + DeserializeOwned` (basically any custom `thiserror` impl)
//! - `Result<T, anyhow::Error>` where we transport the error as a string and/or through ServerFnError
//!
//! The handler return types we support are:
//! - `T where T: FromResponse`
//! - `T where T: DeserializeOwned`
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

use crate::{
    CantEncode, ClientRequest, ClientResponse, EncodeIsVerified, FromResponse, HttpError,
    IntoRequest, ServerFnError,
};
use axum::response::IntoResponse;
use axum_core::extract::{FromRequest, Request};
use bytes::Bytes;
use dioxus_fullstack_core::RequestError;
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

    code: u16,

    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<E>,
}

/// Convert a `RequestError` into a `ServerFnError`.
///
/// This is a separate function to avoid bringing in `reqwest` into fullstack-core.
pub fn reqwest_response_to_serverfn_err(err: reqwest::Error) -> ServerFnError {
    ServerFnError::Request(reqwest_error_to_request_error(err))
}

pub fn reqwest_error_to_request_error(err: reqwest::Error) -> RequestError {
    let message = err.to_string();
    if err.is_timeout() {
        RequestError::Timeout(message)
    } else if err.is_request() {
        RequestError::Request(message)
    } else if err.is_body() {
        RequestError::Body(message)
    } else if err.is_decode() {
        RequestError::Decode(message)
    } else if err.is_redirect() {
        RequestError::Redirect(message)
    } else if let Some(status) = err.status() {
        RequestError::Status(message, status.as_u16())
    } else {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if err.is_connect() {
                RequestError::Connect(message)
            } else {
                RequestError::Request(message)
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            RequestError::Request(message)
        }
    }
}

pub use req_to::*;
pub mod req_to {
    use super::*;

    pub trait EncodeRequest<In, Out, R> {
        type VerifyEncode;
        fn fetch_client(
            &self,
            ctx: ClientRequest,
            data: In,
            map: fn(In) -> Out,
        ) -> impl Future<Output = Result<R, RequestError>> + 'static;
        fn verify_can_serialize(&self) -> Self::VerifyEncode;
    }

    /// Using the deserialize path
    impl<T, O> EncodeRequest<T, O, ClientResponse> for &&&&&&&&&&ServerFnEncoder<T, O>
    where
        T: DeserializeOwned + Serialize + 'static,
    {
        type VerifyEncode = EncodeIsVerified;
        fn fetch_client(
            &self,
            ctx: ClientRequest,
            data: T,
            _map: fn(T) -> O,
        ) -> impl Future<Output = Result<ClientResponse, RequestError>> + 'static {
            async move { ctx.send_json(&data).await }
        }

        fn verify_can_serialize(&self) -> Self::VerifyEncode {
            EncodeIsVerified
        }
    }

    /// When we use the FromRequest path, we don't need to deserialize the input type on the client,
    impl<T, O, R> EncodeRequest<T, O, R> for &&&&&&&&&ServerFnEncoder<T, O>
    where
        T: 'static,
        O: IntoRequest<R>,
    {
        type VerifyEncode = EncodeIsVerified;
        fn fetch_client(
            &self,
            ctx: ClientRequest,
            data: T,
            map: fn(T) -> O,
        ) -> impl Future<Output = Result<R, RequestError>> + 'static {
            O::into_request(map(data), ctx)
        }

        fn verify_can_serialize(&self) -> Self::VerifyEncode {
            EncodeIsVerified
        }
    }

    /// The fall-through case that emits a `CantEncode` type which fails to compile when checked by the macro
    impl<T, O> EncodeRequest<T, O, ClientResponse> for &ServerFnEncoder<T, O>
    where
        T: 'static,
    {
        type VerifyEncode = CantEncode;
        #[allow(clippy::manual_async_fn)]
        fn fetch_client(
            &self,
            _ctx: ClientRequest,
            _data: T,
            _map: fn(T) -> O,
        ) -> impl Future<Output = Result<ClientResponse, RequestError>> + 'static {
            async move { unimplemented!() }
        }

        fn verify_can_serialize(&self) -> Self::VerifyEncode {
            CantEncode
        }
    }
}

pub use decode_ok::*;
mod decode_ok {

    use crate::{CantDecode, DecodeIsVerified};

    use super::*;

    /// Convert the reqwest response into the desired type, in place.
    /// The point here is to prefer FromResponse types *first* and then DeserializeOwned types second.
    ///
    /// This is because FromResponse types are more specialized and can handle things like websockets and files.
    /// DeserializeOwned types are more general and can handle things like JSON responses.
    pub trait RequestDecodeResult<T, R> {
        type VerifyDecode;
        fn decode_client_response(
            &self,
            res: Result<R, RequestError>,
        ) -> impl Future<Output = Result<Result<T, ServerFnError>, RequestError>> + Send;
        fn verify_can_deserialize(&self) -> Self::VerifyDecode;
    }

    impl<T: FromResponse<R>, E, R> RequestDecodeResult<T, R> for &&&ServerFnDecoder<Result<T, E>> {
        type VerifyDecode = DecodeIsVerified;
        fn decode_client_response(
            &self,
            res: Result<R, RequestError>,
        ) -> impl Future<Output = Result<Result<T, ServerFnError>, RequestError>> + Send {
            SendWrapper::new(async move {
                match res {
                    Err(err) => Err(err),
                    Ok(res) => Ok(T::from_response(res).await),
                }
            })
        }
        fn verify_can_deserialize(&self) -> Self::VerifyDecode {
            DecodeIsVerified
        }
    }

    impl<T: DeserializeOwned, E> RequestDecodeResult<T, ClientResponse>
        for &&ServerFnDecoder<Result<T, E>>
    {
        type VerifyDecode = DecodeIsVerified;
        fn decode_client_response(
            &self,
            res: Result<ClientResponse, RequestError>,
        ) -> impl Future<Output = Result<Result<T, ServerFnError>, RequestError>> + Send {
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
                            serde_json::from_slice::<T>(as_bytes)
                                .map(RestEndpointPayload::Success)
                                .map_err(|e| ServerFnError::Deserialization(e.to_string()))
                        } else {
                            match serde_json::from_slice::<ErrorPayload<serde_json::Value>>(
                                as_bytes,
                            ) {
                                Ok(res) => Ok(RestEndpointPayload::Error(ErrorPayload {
                                    message: res.message,
                                    code: res.code,
                                    data: res.data,
                                })),
                                Err(err) => {
                                    if let Ok(text) = String::from_utf8(as_bytes.to_vec()) {
                                        Ok(RestEndpointPayload::Error(ErrorPayload {
                                            message: format!("HTTP {}: {}", status.as_u16(), text),
                                            code: status.as_u16(),
                                            data: None,
                                        }))
                                    } else {
                                        Err(ServerFnError::Deserialization(err.to_string()))
                                    }
                                }
                            }
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
                            Err(e) => Ok(Err(e)),
                        }
                    }
                }
            })
        }
        fn verify_can_deserialize(&self) -> Self::VerifyDecode {
            DecodeIsVerified
        }
    }

    impl<T, R, E> RequestDecodeResult<T, R> for &ServerFnDecoder<Result<T, E>> {
        type VerifyDecode = CantDecode;

        fn decode_client_response(
            &self,
            _res: Result<R, RequestError>,
        ) -> impl Future<Output = Result<Result<T, ServerFnError>, RequestError>> + Send {
            async move { unimplemented!() }
        }

        fn verify_can_deserialize(&self) -> Self::VerifyDecode {
            CantDecode
        }
    }

    pub trait RequestDecodeErr<T, E> {
        fn decode_client_err(
            &self,
            res: Result<Result<T, ServerFnError>, RequestError>,
        ) -> impl Future<Output = Result<T, E>> + Send;
    }

    impl<T, E> RequestDecodeErr<T, E> for &&&ServerFnDecoder<Result<T, E>>
    where
        E: From<ServerFnError> + DeserializeOwned + Serialize,
    {
        fn decode_client_err(
            &self,
            res: Result<Result<T, ServerFnError>, RequestError>,
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
                    Err(err) => Err(ServerFnError::from(err).into()),
                }
            })
        }
    }

    /// Here we convert to ServerFnError and then into the anyhow::Error, letting the user downcast
    /// from the ServerFnError if they want to.
    ///
    /// This loses any actual type information, but is the most flexible for users.
    impl<T> RequestDecodeErr<T, anyhow::Error> for &&ServerFnDecoder<Result<T, anyhow::Error>> {
        fn decode_client_err(
            &self,
            res: Result<Result<T, ServerFnError>, RequestError>,
        ) -> impl Future<Output = Result<T, anyhow::Error>> + Send {
            SendWrapper::new(async move {
                match res {
                    Ok(Ok(res)) => Ok(res),
                    Ok(Err(e)) => Err(anyhow::Error::from(e)),
                    Err(err) => Err(anyhow::Error::from(err)),
                }
            })
        }
    }

    /// This converts to statuscode, which can be useful but loses a lot of information.
    impl<T> RequestDecodeErr<T, StatusCode> for &ServerFnDecoder<Result<T, StatusCode>> {
        fn decode_client_err(
            &self,
            res: Result<Result<T, ServerFnError>, RequestError>,
        ) -> impl Future<Output = Result<T, StatusCode>> + Send {
            SendWrapper::new(async move {
                match res {
                    Ok(Ok(res)) => Ok(res),

                    // We do a best-effort conversion from ServerFnError to StatusCode.
                    Ok(Err(e)) => match e {
                        ServerFnError::Request(error) => {
                            Err(StatusCode::from_u16(error.status_code().unwrap_or(500))
                                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
                        }

                        ServerFnError::ServerError {
                            message: _message,
                            details: _details,
                            code,
                        } => {
                            Err(StatusCode::from_u16(code)
                                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
                        }

                        ServerFnError::Registration(_) | ServerFnError::MiddlewareError(_) => {
                            Err(StatusCode::INTERNAL_SERVER_ERROR)
                        }

                        ServerFnError::Deserialization(_)
                        | ServerFnError::Serialization(_)
                        | ServerFnError::Args(_)
                        | ServerFnError::MissingArg(_)
                        | ServerFnError::StreamError(_) => Err(StatusCode::UNPROCESSABLE_ENTITY),

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

    impl<T> RequestDecodeErr<T, HttpError> for &ServerFnDecoder<Result<T, HttpError>> {
        fn decode_client_err(
            &self,
            res: Result<Result<T, ServerFnError>, RequestError>,
        ) -> impl Future<Output = Result<T, HttpError>> + Send {
            SendWrapper::new(async move {
                match res {
                    Ok(Ok(res)) => Ok(res),
                    Ok(Err(res)) => match res {
                        ServerFnError::ServerError { message, code, .. } => Err(HttpError {
                            status: StatusCode::from_u16(code)
                                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                            message: Some(message),
                        }),
                        _ => HttpError::internal_server_error("Internal Server Error"),
                    },
                    Err(err) => Err(HttpError::new(
                        err.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                        err.to_string(),
                    )),
                }
            })
        }
    }
}

pub use req_from::*;
pub mod req_from {
    use super::*;
    use axum::{extract::FromRequestParts, response::Response};
    use dioxus_fullstack_core::FullstackContext;

    pub trait ExtractRequest<In, Out, H, M = ()> {
        fn extract_axum(
            &self,
            state: FullstackContext,
            request: Request,
            map: fn(In) -> Out,
        ) -> impl Future<Output = Result<(Out, H), Response>> + 'static;
    }

    /// When you're extracting entirely on the server, we need to reject client-consuning request bodies
    /// This sits above priority in the combined headers on server / body on client case.
    impl<In, M, H> ExtractRequest<In, (), H, M> for &&&&&&&&&&&ServerFnEncoder<In, ()>
    where
        H: FromRequest<FullstackContext, M> + 'static,
    {
        fn extract_axum(
            &self,
            state: FullstackContext,
            request: Request,
            _map: fn(In) -> (),
        ) -> impl Future<Output = Result<((), H), Response>> + 'static {
            async move {
                H::from_request(request, &state)
                    .await
                    .map_err(|e| e.into_response())
                    .map(|out| ((), out))
            }
        }
    }

    // One-arg case
    impl<In, Out, H> ExtractRequest<In, Out, H> for &&&&&&&&&&ServerFnEncoder<In, Out>
    where
        In: DeserializeOwned + 'static,
        Out: 'static,
        H: FromRequestParts<FullstackContext>,
    {
        fn extract_axum(
            &self,
            _state: FullstackContext,
            request: Request,
            map: fn(In) -> Out,
        ) -> impl Future<Output = Result<(Out, H), Response>> + 'static {
            async move {
                let (mut parts, body) = request.into_parts();
                let headers = H::from_request_parts(&mut parts, &_state)
                    .await
                    .map_err(|e| e.into_response())?;

                let request = Request::from_parts(parts, body);
                let bytes = Bytes::from_request(request, &()).await.unwrap();
                let as_str = String::from_utf8_lossy(&bytes);

                let bytes = if as_str.is_empty() {
                    "{}".as_bytes()
                } else {
                    &bytes
                };

                let out = serde_json::from_slice::<In>(bytes)
                    .map(map)
                    .map_err(|e| ServerFnError::from(e).into_response())?;

                Ok((out, headers))
            }
        }
    }

    /// We skip the BodySerialize wrapper and just go for the output type directly.
    impl<In, Out, M, H> ExtractRequest<In, Out, H, M> for &&&&&&&&&ServerFnEncoder<In, Out>
    where
        Out: FromRequest<FullstackContext, M> + 'static,
        H: FromRequestParts<FullstackContext>,
    {
        fn extract_axum(
            &self,
            state: FullstackContext,
            request: Request,
            _map: fn(In) -> Out,
        ) -> impl Future<Output = Result<(Out, H), Response>> + 'static {
            async move {
                let (mut parts, body) = request.into_parts();
                let headers = H::from_request_parts(&mut parts, &state)
                    .await
                    .map_err(|e| e.into_response())?;

                let request = Request::from_parts(parts, body);

                let res = Out::from_request(request, &state)
                    .await
                    .map_err(|e| e.into_response());

                res.map(|out| (out, headers))
            }
        }
    }
}

pub use resp::*;
mod resp {
    use crate::HttpError;

    use super::*;
    use axum::response::Response;
    use dioxus_core::CapturedError;
    use http::HeaderValue;

    /// A trait for converting the result of the Server Function into an Axum response.
    ///
    /// This is to work around the issue where we want to return both Deserialize types and FromResponse types.
    /// Stuff like websockets
    ///
    /// We currently have an `Input` type even though it's not useful since we might want to support regular axum endpoints later.
    /// For now, it's just Result<T, E> where T is either DeserializeOwned or FromResponse
    pub trait MakeAxumResponse<T, E, R> {
        fn make_axum_response(self, result: Result<T, E>) -> Result<Response, E>;
    }

    // Higher priority impl for special types like websocket/file responses that generate their own responses
    // The FromResponse impl helps narrow types to those usable on the client
    impl<T, E, R> MakeAxumResponse<T, E, R> for &&&&ServerFnDecoder<Result<T, E>>
    where
        T: FromResponse<R> + IntoResponse,
    {
        fn make_axum_response(self, result: Result<T, E>) -> Result<Response, E> {
            result.map(|v| v.into_response())
        }
    }

    // Lower priority impl for regular serializable types
    // We try to match the encoding from the incoming request, otherwise default to JSON
    impl<T, E> MakeAxumResponse<T, E, ()> for &&&ServerFnDecoder<Result<T, E>>
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
        fn make_axum_error(self, result: Result<Response, E>) -> Response;
    }

    /// Get the status code from the error type if possible.
    pub trait AsStatusCode {
        fn as_status_code(&self) -> StatusCode;
    }

    impl AsStatusCode for ServerFnError {
        fn as_status_code(&self) -> StatusCode {
            match self {
                Self::ServerError { code, .. } => {
                    StatusCode::from_u16(*code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
                }
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        }
    }

    impl<T, E> MakeAxumError<E> for &&&ServerFnDecoder<Result<T, E>>
    where
        E: AsStatusCode + From<ServerFnError> + Serialize + DeserializeOwned + Display,
    {
        fn make_axum_error(self, result: Result<Response, E>) -> Response {
            match result {
                Ok(res) => res,
                Err(err) => {
                    let status_code = err.as_status_code();
                    let err = ErrorPayload {
                        code: status_code.as_u16(),
                        message: err.to_string(),
                        data: Some(err),
                    };
                    let body = serde_json::to_string(&err).unwrap();
                    let mut resp = Response::new(body.into());
                    resp.headers_mut().insert(
                        http::header::CONTENT_TYPE,
                        HeaderValue::from_static("application/json"),
                    );
                    *resp.status_mut() = status_code;
                    resp
                }
            }
        }
    }

    impl<T> MakeAxumError<CapturedError> for &&ServerFnDecoder<Result<T, CapturedError>> {
        fn make_axum_error(self, result: Result<Response, CapturedError>) -> Response {
            match result {
                Ok(res) => res,

                // Optimize the case where we have sole ownership of the error
                Err(errr) if errr._strong_count() == 1 => {
                    let err = errr.into_inner().unwrap();
                    <&&ServerFnDecoder<Result<T, anyhow::Error>> as MakeAxumError<anyhow::Error>>::make_axum_error(
                        &&ServerFnDecoder::new(),
                        Err(err),
                    )
                }

                Err(errr) => {
                    // The `WithHttpError` trait emits ServerFnErrors so we can downcast them here
                    // to create richer responses.
                    let payload = match errr.downcast_ref::<ServerFnError>() {
                        Some(ServerFnError::ServerError {
                            message,
                            code,
                            details,
                        }) => ErrorPayload {
                            message: message.clone(),
                            code: *code,
                            data: details.clone(),
                        },
                        Some(other) => ErrorPayload {
                            message: other.to_string(),
                            code: 500,
                            data: None,
                        },
                        None => match errr.downcast_ref::<HttpError>() {
                            Some(http_err) => ErrorPayload {
                                message: http_err
                                    .message
                                    .clone()
                                    .unwrap_or_else(|| http_err.status.to_string()),
                                code: http_err.status.as_u16(),
                                data: None,
                            },
                            None => ErrorPayload {
                                code: 500,
                                message: errr.to_string(),
                                data: None,
                            },
                        },
                    };

                    let body = serde_json::to_string(&payload).unwrap();
                    let mut resp = Response::new(body.into());
                    resp.headers_mut().insert(
                        http::header::CONTENT_TYPE,
                        HeaderValue::from_static("application/json"),
                    );
                    *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                    resp
                }
            }
        }
    }

    impl<T> MakeAxumError<anyhow::Error> for &&ServerFnDecoder<Result<T, anyhow::Error>> {
        fn make_axum_error(self, result: Result<Response, anyhow::Error>) -> Response {
            match result {
                Ok(res) => res,
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
                            code: 500,
                            data: None,
                        },
                        Err(err) => match err.downcast::<HttpError>() {
                            Ok(http_err) => ErrorPayload {
                                message: http_err
                                    .message
                                    .unwrap_or_else(|| http_err.status.to_string()),
                                code: http_err.status.as_u16(),
                                data: None,
                            },
                            Err(err) => ErrorPayload {
                                code: 500,
                                message: err.to_string(),
                                data: None,
                            },
                        },
                    };

                    let body = serde_json::to_string(&payload).unwrap();
                    let mut resp = Response::new(body.into());
                    resp.headers_mut().insert(
                        http::header::CONTENT_TYPE,
                        HeaderValue::from_static("application/json"),
                    );
                    *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                    resp
                }
            }
        }
    }

    impl<T> MakeAxumError<StatusCode> for &&ServerFnDecoder<Result<T, StatusCode>> {
        fn make_axum_error(self, result: Result<Response, StatusCode>) -> Response {
            match result {
                Ok(resp) => resp,
                Err(status) => {
                    let body = serde_json::to_string(&ErrorPayload::<()> {
                        code: status.as_u16(),
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
                    resp
                }
            }
        }
    }

    impl<T> MakeAxumError<HttpError> for &ServerFnDecoder<Result<T, HttpError>> {
        fn make_axum_error(self, result: Result<Response, HttpError>) -> Response {
            match result {
                Ok(resp) => resp,
                Err(http_err) => {
                    let body = serde_json::to_string(&ErrorPayload::<()> {
                        code: http_err.status.as_u16(),
                        message: http_err
                            .message
                            .unwrap_or_else(|| http_err.status.to_string()),
                        data: None,
                    })
                    .unwrap();
                    let mut resp = Response::new(body.into());
                    resp.headers_mut().insert(
                        http::header::CONTENT_TYPE,
                        HeaderValue::from_static("application/json"),
                    );
                    *resp.status_mut() = http_err.status;
                    resp
                }
            }
        }
    }
}
