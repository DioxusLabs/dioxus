use std::{
    error::Error,
    fmt::{Debug, Display},
    str::FromStr,
};

use dioxus_lib::prelude::dioxus_core::CapturedError;
use serde::{de::DeserializeOwned, Serialize};
use server_fn::{
    codec::JsonEncoding,
    error::{FromServerFnError, ServerFnErrorErr},
};

/// A default result type for server functions, which can either be successful or contain an error.
pub type ServerFnResult<T = (), E = String> = std::result::Result<T, ServerFnError<E>>;

/// An instance of an error captured by a descendant component.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum ServerFnError<T = String> {
    /// An error running the server function
    ServerError(T),

    /// An error communicating with the server
    CommunicationError(ServerFnErrorErr),
}

impl ServerFnError {
    /// Creates a new `ServerFnError` from something that implements `ToString`.
    pub fn new(error: impl ToString) -> Self {
        Self::ServerError(error.to_string())
    }
}

impl<T> ServerFnError<T> {
    /// Returns true if the error is a server error
    pub fn is_server_error(&self) -> bool {
        matches!(self, ServerFnError::ServerError(_))
    }

    /// Returns true if the error is a communication error
    pub fn is_communication_error(&self) -> bool {
        matches!(self, ServerFnError::CommunicationError(_))
    }
}

impl<T: Serialize + DeserializeOwned + Debug + 'static> FromServerFnError for ServerFnError<T> {
    type Encoder = JsonEncoding;

    fn from_server_fn_error(err: ServerFnErrorErr) -> Self {
        Self::CommunicationError(err)
    }
}

impl<T: FromStr> FromStr for ServerFnError<T> {
    type Err = <T as FromStr>::Err;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        std::result::Result::Ok(Self::ServerError(T::from_str(s)?))
    }
}

impl<T: Display> Display for ServerFnError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerFnError::ServerError(err) => write!(f, "Server error: {}", err),
            ServerFnError::CommunicationError(err) => write!(f, "Communication error: {}", err),
        }
    }
}

impl From<ServerFnError> for CapturedError {
    fn from(error: ServerFnError) -> Self {
        Self::from_display(error)
    }
}

impl<E: Error> From<E> for ServerFnError<String> {
    fn from(error: E) -> Self {
        Self::ServerError(error.to_string())
    }
}
