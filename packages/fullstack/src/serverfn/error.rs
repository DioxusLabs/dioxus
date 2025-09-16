use axum::response::IntoResponse;
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

impl From<reqwest::Error> for ServerFnError {
    fn from(value: reqwest::Error) -> Self {
        ServerFnError::Request {
            message: value.to_string(),
            code: value.status().map(|s| s.as_u16()),
        }
    }
}

impl From<anyhow::Error> for ServerFnError {
    fn from(value: anyhow::Error) -> Self {
        ServerFnError::ServerError(value.to_string())
    }
}

#[derive(Debug)]
pub struct ServerFnRejection {}
impl IntoResponse for ServerFnRejection {
    fn into_response(self) -> axum::response::Response {
        todo!()
    }
}

pub trait ServerFnSugar<M> {
    fn desugar_into_response(self) -> axum::response::Response;
    fn from_reqwest(res: reqwest::Response) -> Self
    where
        Self: Sized,
    {
        todo!()
    }
}

// pub trait IntoClientErr<M> {
//     fn try_into_client_err(self) -> Option<ServerFnError>;
// }

// impl<T, E> IntoClientErr<()> for Result<T, E>
// where
//     Self: Into<ServerFnError>,
// {
//     fn try_into_client_err(self) -> Option<ServerFnError> {
//         Some(self.into())
//     }
// }

// pub struct CantGoMarker;
// impl<T> IntoClientErr<CantGoMarker> for &T {
//     fn try_into_client_err(self) -> Option<ServerFnError> {
//         None
//     }
// }
