use axum_core::response::IntoResponse;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

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
#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerFnError {
    /// Occurs when there is an error while actually running the function on the server.
    ///
    /// The `details` field can optionally contain additional structured information about the error.
    /// When passing typed errors from the server to the client, the `details` field contains the serialized
    /// representation of the error.
    #[error("error running server function: {message} (details: {details:#?})")]
    ServerError {
        message: String,

        /// Optional HTTP status code associated with the error.
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<u16>,

        #[serde(skip_serializing_if = "Option::is_none")]
        details: Option<serde_json::Value>,
    },

    /// Occurs on the client if there is a network error while trying to run function on server.
    #[error("error reaching server to call server function: {message} (code: {code:?})")]
    Request { message: String, code: Option<u16> },

    /// Occurs on the client if there is an error while trying to read the response body as a stream.
    #[error("error reading response body stream: {0}")]
    StreamError(String),

    /// Error while trying to register the server function (only occurs in case of poisoned RwLock).
    #[error("error while trying to register the server function: {0}")]
    Registration(String),

    /// Occurs on the client if trying to use an unsupported `HTTP` method when building a request.
    #[error("error trying to build `HTTP` method request: {0}")]
    UnsupportedRequestMethod(String),

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
        ServerFnError::ServerError {
            message: value.to_string(),
            details: None,
            code: None,
        }
    }
}

impl From<ServerFnError> for http::StatusCode {
    fn from(value: ServerFnError) -> Self {
        match value {
            ServerFnError::ServerError { code, .. } => match code {
                Some(code) => http::StatusCode::from_u16(code)
                    .unwrap_or(http::StatusCode::INTERNAL_SERVER_ERROR),
                None => http::StatusCode::INTERNAL_SERVER_ERROR,
            },
            ServerFnError::Request { code, .. } => match code {
                Some(code) => http::StatusCode::from_u16(code)
                    .unwrap_or(http::StatusCode::INTERNAL_SERVER_ERROR),
                None => http::StatusCode::INTERNAL_SERVER_ERROR,
            },
            ServerFnError::StreamError(_)
            | ServerFnError::Registration(_)
            | ServerFnError::UnsupportedRequestMethod(_)
            | ServerFnError::MiddlewareError(_)
            | ServerFnError::Deserialization(_)
            | ServerFnError::Serialization(_)
            | ServerFnError::Args(_)
            | ServerFnError::MissingArg(_)
            | ServerFnError::Response(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Debug)]
pub struct ServerFnRejection {}
impl IntoResponse for ServerFnRejection {
    fn into_response(self) -> axum_core::response::Response {
        axum_core::response::Response::builder()
            .status(http::StatusCode::INTERNAL_SERVER_ERROR)
            .body(axum_core::body::Body::from("Internal Server Error"))
            .unwrap()
    }
}
