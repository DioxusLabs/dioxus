use http::StatusCode;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use crate::HttpError;

/// A default result type for server functions, which can either be successful or contain an error. The [`ServerFnResult`] type
/// is a convenient alias for a `Result` type that uses [`ServerFnError`] as the error type.
///
/// # Example
/// ```rust
/// use dioxus::prelude::*;
///
/// #[server]
/// async fn parse_number(number: String) -> Result<f32> {
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
        /// A human-readable message describing the error.
        message: String,

        /// HTTP status code associated with the error.
        code: u16,

        #[serde(skip_serializing_if = "Option::is_none")]
        details: Option<serde_json::Value>,
    },

    /// Occurs on the client if there is a network error while trying to run function on server.
    #[error("error reaching server to call server function: {0} ")]
    Request(RequestError),

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

impl ServerFnError {
    /// Create a new server error (status code 500) with a message.
    pub fn new(f: impl ToString) -> Self {
        ServerFnError::ServerError {
            message: f.to_string(),
            details: None,
            code: 500,
        }
    }

    /// Create a server error from an HTTP response, using the body as the message and the
    /// response status as the code.
    pub fn from_http_response<B: AsRef<[u8]>>(resp: http::Response<B>) -> Self {
        let status = resp.status();
        let message = std::str::from_utf8(resp.body().as_ref())
            .map(str::to_string)
            .unwrap_or_else(|_| status.canonical_reason().unwrap_or("").to_string());

        ServerFnError::ServerError {
            message,
            code: status.as_u16(),
            details: None,
        }
    }

    /// Convert this error into an HTTP response with a JSON body of the form
    /// `{ "error": message, "details": details }`.
    pub fn into_http_response(self) -> http::Response<String> {
        let (status, body) = match self {
            Self::ServerError {
                message,
                code,
                details,
            } => {
                let status =
                    StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                let body = match details {
                    Some(details) => serde_json::json!({ "error": message, "details": details }),
                    None => serde_json::json!({ "error": message }),
                };
                (status, body)
            }
            other => (
                StatusCode::from(other.clone()),
                serde_json::json!({ "error": other.to_string() }),
            ),
        };

        let mut resp = http::Response::new(body.to_string());
        *resp.status_mut() = status;
        resp.headers_mut().insert(
            http::header::CONTENT_TYPE,
            http::HeaderValue::from_static("application/json"),
        );
        resp
    }
}

impl From<anyhow::Error> for ServerFnError {
    fn from(value: anyhow::Error) -> Self {
        ServerFnError::ServerError {
            message: value.to_string(),
            details: None,
            code: 500,
        }
    }
}

impl From<serde_json::Error> for ServerFnError {
    fn from(value: serde_json::Error) -> Self {
        ServerFnError::Deserialization(value.to_string())
    }
}

impl From<ServerFnError> for http::StatusCode {
    fn from(value: ServerFnError) -> Self {
        match value {
            ServerFnError::ServerError { code, .. } => {
                http::StatusCode::from_u16(code).unwrap_or(http::StatusCode::INTERNAL_SERVER_ERROR)
            }
            ServerFnError::Request(err) => match err {
                RequestError::Status(_, code) => http::StatusCode::from_u16(code)
                    .unwrap_or(http::StatusCode::INTERNAL_SERVER_ERROR),
                _ => http::StatusCode::INTERNAL_SERVER_ERROR,
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

impl From<RequestError> for ServerFnError {
    fn from(value: RequestError) -> Self {
        ServerFnError::Request(value)
    }
}

impl From<ServerFnError> for HttpError {
    fn from(value: ServerFnError) -> Self {
        let status = StatusCode::from_u16(match &value {
            ServerFnError::ServerError { code, .. } => *code,
            _ => 500,
        })
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        HttpError {
            status,
            message: Some(value.to_string()),
        }
    }
}

impl From<HttpError> for ServerFnError {
    fn from(value: HttpError) -> Self {
        ServerFnError::ServerError {
            message: value.message.unwrap_or_else(|| {
                value
                    .status
                    .canonical_reason()
                    .unwrap_or("Unknown error")
                    .to_string()
            }),
            code: value.status.as_u16(),
            details: None,
        }
    }
}

/// An error type representing issues that can occur while making requests.
///
/// This is made to paper over the reqwest::Error type which we don't want to export here and
/// is limited in many ways.
#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestError {
    /// An error occurred when building the request.
    #[error("error building request: {0}")]
    Builder(String),

    /// An error occurred when serializing the request body.
    #[error("error serializing request body: {0}")]
    Serialization(String),

    /// An error occurred when following a redirect.
    #[error("error following redirect: {0}")]
    Redirect(String),

    /// An error occurred when receiving a non-2xx status code.
    #[error("error receiving status code: {0} ({1})")]
    Status(String, u16),

    /// An error occurred when a request times out.
    #[error("error timing out: {0}")]
    Timeout(String),

    /// An error occurred when sending a request.
    #[error("error sending request: {0}")]
    Request(String),

    /// An error occurred when upgrading a connection.
    #[error("error upgrading connection: {0}")]
    Connect(String),

    /// An error occurred when there is a request or response body error.
    #[error("request or response body error: {0}")]
    Body(String),

    /// An error occurred when decoding the response body.
    #[error("error decoding response body: {0}")]
    Decode(String),
}

impl RequestError {
    pub fn status(&self) -> Option<StatusCode> {
        match self {
            RequestError::Status(_, code) => Some(StatusCode::from_u16(*code).ok()?),
            _ => None,
        }
    }

    pub fn status_code(&self) -> Option<u16> {
        match self {
            RequestError::Status(_, code) => Some(*code),
            _ => None,
        }
    }
}
