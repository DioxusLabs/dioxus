use axum_core::response::{IntoResponse, Response};
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
// use crate::{ContentType, Decodes, Encodes, Format, FormatType};

/// A default result type for server functions, which can either be successful or contain an error. The [`ServerFnResult`] type
/// is a convenient alias for a `Result` type that uses [`ServerFnError`] as the error type.
///
/// # Example
/// ```rust
/// use dioxus::prelude::*;
///
/// #[server]
/// async fn parse_number(number: String) -> ServerFnResult<f32> {
///     let parsed_number: f32 = number.parse()?;
///     Ok(parsed_number)
/// }
/// ```
pub type ServerFnResult<T = ()> = std::result::Result<T, ServerFnError>;

/// The error type for the server function system. This enum encompasses all possible errors that can occur
/// during the registration, invocation, and processing of server functions.
///
///
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerFnError {
    /// Occurs when there is an error while actually running the function on the server.
    #[error("error running server function: {0}")]
    ServerError(String),

    /// Error while trying to register the server function (only occurs in case of poisoned RwLock).
    #[error("error while trying to register the server function: {0}")]
    Registration(String),

    /// Occurs on the client if trying to use an unsupported `HTTP` method when building a request.
    #[error("error trying to build `HTTP` method request: {0}")]
    UnsupportedRequestMethod(String),

    /// Occurs on the client if there is a network error while trying to run function on server.
    #[error("error reaching server to call server function: {message} (code: {code:?})")]
    Request { message: String, code: Option<u16> },

    /// Occurs when there is an error while actually running the middleware on the server.
    #[error("error running middleware: {0}")]
    MiddlewareError(String),

    /// Occurs on the client if there is an error deserializing the server's response.
    #[error("error deserializing server function results: {0}")]
    Deserialization(String),

    /// Occurs on the client if there is an error serializing the server function arguments.
    #[error("error serializing server function arguments: {0}")]
    Serialization(String),

    /// Occurs on the server if there is an error deserializing one of the arguments that's been sent.
    #[error("error deserializing server function arguments: {0}")]
    Args(String),

    /// Occurs on the server if there's a missing argument.
    #[error("missing argument {0}")]
    MissingArg(String),

    /// Occurs on the server if there is an error creating an HTTP response.
    #[error("error creating response {0}")]
    Response(String),
}

impl From<anyhow::Error> for ServerFnError {
    fn from(value: anyhow::Error) -> Self {
        ServerFnError::ServerError(value.to_string())
    }
}

#[derive(Debug)]
pub struct ServerFnRejection {}
impl IntoResponse for ServerFnRejection {
    fn into_response(self) -> axum_core::response::Response {
        todo!()
    }
}

pub trait ServerFnSugar<M> {
    fn desugar_into_response(self) -> axum_core::response::Response;
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
        match self {
            Self::Ok(e) => e.into_response(),
            Self::Err(e) => e.to_encode_response(),
        }
    }
}

/// This covers the simple case of returning a body from an endpoint where the body is serializable.
/// By default, we use the JSON encoding, but you can use one of the other newtypes to change the encoding.
pub struct DefaultJsonEncodingMarker;
impl<T: Serialize, E: IntoResponse> ServerFnSugar<DefaultJsonEncodingMarker> for &Result<T, E> {
    fn desugar_into_response(self) -> Response {
        match self.as_ref() {
            Ok(e) => {
                let body = serde_json::to_vec(e).unwrap();
                (http::StatusCode::OK, body).into_response()
            }
            Err(e) => todo!(),
        }
    }
}

pub struct SerializeSugarWithErrorMarker;
impl<T: Serialize, E: ErrorSugar> ServerFnSugar<SerializeSugarWithErrorMarker> for &Result<T, E> {
    fn desugar_into_response(self) -> Response {
        match self.as_ref() {
            Ok(e) => {
                let body = serde_json::to_vec(e).unwrap();
                (http::StatusCode::OK, body).into_response()
            }
            Err(e) => e.to_encode_response(),
        }
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
