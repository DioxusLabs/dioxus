#![allow(deprecated)]

use crate::{ContentType, Decodes, Encodes, Format, FormatType};
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display, Write},
    str::FromStr,
};
use throw_error::Error;
use url::Url;

/// A custom header that can be used to indicate a server function returned an error.
pub const SERVER_FN_ERROR_HEADER: &str = "serverfnerror";

impl From<ServerFnError> for Error {
    fn from(e: ServerFnError) -> Self {
        Error::from(ServerFnErrorWrapper(e))
    }
}

/// An empty value indicating that there is no custom error type associated
/// with this server function.
#[derive(
    Debug,
    Deserialize,
    Serialize,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Clone,
    Copy,
)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[deprecated(
    since = "0.8.0",
    note = "Now server_fn can return any error type other than ServerFnError, \
            so the WrappedServerError variant will be removed in 0.9.0"
)]
pub struct NoCustomError;

// Implement `Display` for `NoCustomError`
impl fmt::Display for NoCustomError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Unit Type Displayed")
    }
}

impl FromStr for NoCustomError {
    type Err = ();

    fn from_str(_s: &str) -> Result<Self, Self::Err> {
        Ok(NoCustomError)
    }
}

/// Wraps some error type, which may implement any of [`Error`](trait@std::error::Error), [`Clone`], or
/// [`Display`].
#[derive(Debug)]
#[deprecated(
    since = "0.8.0",
    note = "Now server_fn can return any error type other than ServerFnError, \
            so the WrappedServerError variant will be removed in 0.9.0"
)]
pub struct WrapError<T>(pub T);

/// A helper macro to convert a variety of different types into `ServerFnError`.
/// This should mostly be used if you are implementing `From<ServerFnError>` for `YourError`.
#[macro_export]
#[deprecated(
    since = "0.8.0",
    note = "Now server_fn can return any error type other than ServerFnError, \
            so the WrappedServerError variant will be removed in 0.9.0"
)]
macro_rules! server_fn_error {
    () => {{
        use $crate::{ViaError, WrapError};
        (&&&&&WrapError(())).to_server_error()
    }};
    ($err:expr) => {{
        use $crate::error::{ViaError, WrapError};
        match $err {
            error => (&&&&&WrapError(error)).to_server_error(),
        }
    }};
}

/// This trait serves as the conversion method between a variety of types
/// and [`ServerFnError`].
#[deprecated(
    since = "0.8.0",
    note = "Now server_fn can return any error type other than ServerFnError, \
            so users should place their custom error type instead of \
            ServerFnError"
)]
pub trait ViaError<E> {
    /// Converts something into an error.
    fn to_server_error(&self) -> ServerFnError<E>;
}

// This impl should catch if you fed it a [`ServerFnError`] already.
impl<E: ServerFnErrorKind + std::error::Error + Clone> ViaError<E>
    for &&&&WrapError<ServerFnError<E>>
{
    fn to_server_error(&self) -> ServerFnError<E> {
        self.0.clone()
    }
}

// A type tag for ServerFnError so we can special case it
#[deprecated]
pub(crate) trait ServerFnErrorKind {}

impl ServerFnErrorKind for ServerFnError {}

// This impl should catch passing () or nothing to server_fn_error
impl ViaError<NoCustomError> for &&&WrapError<()> {
    fn to_server_error(&self) -> ServerFnError {
        ServerFnError::WrappedServerError(NoCustomError)
    }
}

// This impl will catch any type that implements any type that impls
// Error and Clone, so that it can be wrapped into ServerFnError
impl<E: std::error::Error + Clone> ViaError<E> for &&WrapError<E> {
    fn to_server_error(&self) -> ServerFnError<E> {
        ServerFnError::WrappedServerError(self.0.clone())
    }
}

// If it doesn't impl Error, but does impl Display and Clone,
// we can still wrap it in String form
impl<E: Display + Clone> ViaError<E> for &WrapError<E> {
    fn to_server_error(&self) -> ServerFnError<E> {
        ServerFnError::ServerError(self.0.to_string())
    }
}

// This is what happens if someone tries to pass in something that does
// not meet the above criteria
impl<E> ViaError<E> for WrapError<E> {
    #[track_caller]
    fn to_server_error(&self) -> ServerFnError<E> {
        panic!(
            "At {}, you call `to_server_error()` or use  `server_fn_error!` \
             with a value that does not implement `Clone` and either `Error` \
             or `Display`.",
            std::panic::Location::caller()
        );
    }
}

/// A type that can be used as the return type of the server function for easy error conversion with `?` operator.
/// This type can be replaced with any other error type that implements `FromServerFnError`.
///
/// Unlike [`ServerFnErrorErr`], this does not implement [`Error`](trait@std::error::Error).
/// This means that other error types can easily be converted into it using the
/// `?` operator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub enum ServerFnError<E = NoCustomError> {
    #[deprecated(
        since = "0.8.0",
        note = "Now server_fn can return any error type other than \
                ServerFnError, so users should place their custom error type \
                instead of ServerFnError"
    )]
    /// A user-defined custom error type, which defaults to [`NoCustomError`].
    WrappedServerError(E),
    /// Error while trying to register the server function (only occurs in case of poisoned RwLock).
    Registration(String),
    /// Occurs on the client if there is a network error while trying to run function on server.
    Request(String),
    /// Occurs on the server if there is an error creating an HTTP response.
    Response(String),
    /// Occurs when there is an error while actually running the function on the server.
    ServerError(String),
    /// Occurs when there is an error while actually running the middleware on the server.
    MiddlewareError(String),
    /// Occurs on the client if there is an error deserializing the server's response.
    Deserialization(String),
    /// Occurs on the client if there is an error serializing the server function arguments.
    Serialization(String),
    /// Occurs on the server if there is an error deserializing one of the arguments that's been sent.
    Args(String),
    /// Occurs on the server if there's a missing argument.
    MissingArg(String),
}

impl ServerFnError<NoCustomError> {
    /// Constructs a new [`ServerFnError::ServerError`] from some other type.
    pub fn new(msg: impl ToString) -> Self {
        Self::ServerError(msg.to_string())
    }
}

impl<CustErr> From<CustErr> for ServerFnError<CustErr> {
    fn from(value: CustErr) -> Self {
        ServerFnError::WrappedServerError(value)
    }
}

impl<E: std::error::Error> From<E> for ServerFnError {
    fn from(value: E) -> Self {
        ServerFnError::ServerError(value.to_string())
    }
}

impl<CustErr> Display for ServerFnError<CustErr>
where
    CustErr: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ServerFnError::Registration(s) => format!(
                    "error while trying to register the server function: {s}"
                ),
                ServerFnError::Request(s) => format!(
                    "error reaching server to call server function: {s}"
                ),
                ServerFnError::ServerError(s) =>
                    format!("error running server function: {s}"),
                ServerFnError::MiddlewareError(s) =>
                    format!("error running middleware: {s}"),
                ServerFnError::Deserialization(s) =>
                    format!("error deserializing server function results: {s}"),
                ServerFnError::Serialization(s) =>
                    format!("error serializing server function arguments: {s}"),
                ServerFnError::Args(s) => format!(
                    "error deserializing server function arguments: {s}"
                ),
                ServerFnError::MissingArg(s) => format!("missing argument {s}"),
                ServerFnError::Response(s) =>
                    format!("error generating HTTP response: {s}"),
                ServerFnError::WrappedServerError(e) => format!("{e}"),
            }
        )
    }
}

/// Serializes and deserializes JSON with [`serde_json`].
pub struct ServerFnErrorEncoding;

impl ContentType for ServerFnErrorEncoding {
    const CONTENT_TYPE: &'static str = "text/plain";
}

impl FormatType for ServerFnErrorEncoding {
    const FORMAT_TYPE: Format = Format::Text;
}

impl<CustErr> Encodes<ServerFnError<CustErr>> for ServerFnErrorEncoding
where
    CustErr: Display,
{
    type Error = std::fmt::Error;

    fn encode(output: &ServerFnError<CustErr>) -> Result<Bytes, Self::Error> {
        let mut buf = String::new();
        let result = match output {
            ServerFnError::WrappedServerError(e) => {
                write!(&mut buf, "WrappedServerFn|{e}")
            }
            ServerFnError::Registration(e) => {
                write!(&mut buf, "Registration|{e}")
            }
            ServerFnError::Request(e) => write!(&mut buf, "Request|{e}"),
            ServerFnError::Response(e) => write!(&mut buf, "Response|{e}"),
            ServerFnError::ServerError(e) => {
                write!(&mut buf, "ServerError|{e}")
            }
            ServerFnError::MiddlewareError(e) => {
                write!(&mut buf, "MiddlewareError|{e}")
            }
            ServerFnError::Deserialization(e) => {
                write!(&mut buf, "Deserialization|{e}")
            }
            ServerFnError::Serialization(e) => {
                write!(&mut buf, "Serialization|{e}")
            }
            ServerFnError::Args(e) => write!(&mut buf, "Args|{e}"),
            ServerFnError::MissingArg(e) => {
                write!(&mut buf, "MissingArg|{e}")
            }
        };

        match result {
            Ok(()) => Ok(Bytes::from(buf)),
            Err(e) => Err(e),
        }
    }
}

impl<CustErr> Decodes<ServerFnError<CustErr>> for ServerFnErrorEncoding
where
    CustErr: FromStr,
{
    type Error = String;

    fn decode(bytes: Bytes) -> Result<ServerFnError<CustErr>, Self::Error> {
        let data = String::from_utf8(bytes.to_vec())
            .map_err(|err| format!("UTF-8 conversion error: {err}"))?;

        data.split_once('|')
            .ok_or_else(|| {
                format!("Invalid format: missing delimiter in {data:?}")
            })
            .and_then(|(ty, data)| match ty {
                "WrappedServerFn" => CustErr::from_str(data)
                    .map(ServerFnError::WrappedServerError)
                    .map_err(|_| {
                        format!("Failed to parse CustErr from {data:?}")
                    }),
                "Registration" => {
                    Ok(ServerFnError::Registration(data.to_string()))
                }
                "Request" => Ok(ServerFnError::Request(data.to_string())),
                "Response" => Ok(ServerFnError::Response(data.to_string())),
                "ServerError" => {
                    Ok(ServerFnError::ServerError(data.to_string()))
                }
                "MiddlewareError" => {
                    Ok(ServerFnError::MiddlewareError(data.to_string()))
                }
                "Deserialization" => {
                    Ok(ServerFnError::Deserialization(data.to_string()))
                }
                "Serialization" => {
                    Ok(ServerFnError::Serialization(data.to_string()))
                }
                "Args" => Ok(ServerFnError::Args(data.to_string())),
                "MissingArg" => Ok(ServerFnError::MissingArg(data.to_string())),
                _ => Err(format!("Unknown error type: {ty}")),
            })
    }
}

impl<CustErr> FromServerFnError for ServerFnError<CustErr>
where
    CustErr: std::fmt::Debug + Display + FromStr + 'static,
{
    type Encoder = ServerFnErrorEncoding;

    fn from_server_fn_error(value: ServerFnErrorErr) -> Self {
        match value {
            ServerFnErrorErr::Registration(value) => {
                ServerFnError::Registration(value)
            }
            ServerFnErrorErr::Request(value) => ServerFnError::Request(value),
            ServerFnErrorErr::ServerError(value) => {
                ServerFnError::ServerError(value)
            }
            ServerFnErrorErr::MiddlewareError(value) => {
                ServerFnError::MiddlewareError(value)
            }
            ServerFnErrorErr::Deserialization(value) => {
                ServerFnError::Deserialization(value)
            }
            ServerFnErrorErr::Serialization(value) => {
                ServerFnError::Serialization(value)
            }
            ServerFnErrorErr::Args(value) => ServerFnError::Args(value),
            ServerFnErrorErr::MissingArg(value) => {
                ServerFnError::MissingArg(value)
            }
            ServerFnErrorErr::Response(value) => ServerFnError::Response(value),
            ServerFnErrorErr::UnsupportedRequestMethod(value) => {
                ServerFnError::Request(value)
            }
        }
    }
}

impl<E> std::error::Error for ServerFnError<E>
where
    E: std::error::Error + 'static,
    ServerFnError<E>: std::fmt::Display,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ServerFnError::WrappedServerError(e) => Some(e),
            _ => None,
        }
    }
}

/// Type for errors that can occur when using server functions. If you need to return a custom error type from a server function, implement `FromServerFnError` for your custom error type.
#[derive(
    thiserror::Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize,
)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub enum ServerFnErrorErr {
    /// Error while trying to register the server function (only occurs in case of poisoned RwLock).
    #[error("error while trying to register the server function: {0}")]
    Registration(String),
    /// Occurs on the client if trying to use an unsupported `HTTP` method when building a request.
    #[error("error trying to build `HTTP` method request: {0}")]
    UnsupportedRequestMethod(String),
    /// Occurs on the client if there is a network error while trying to run function on server.
    #[error("error reaching server to call server function: {0}")]
    Request(String),
    /// Occurs when there is an error while actually running the function on the server.
    #[error("error running server function: {0}")]
    ServerError(String),
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

/// Associates a particular server function error with the server function
/// found at a particular path.
///
/// This can be used to pass an error from the server back to the client
/// without JavaScript/WASM supported, by encoding it in the URL as a query string.
/// This is useful for progressive enhancement.
#[derive(Debug)]
pub struct ServerFnUrlError<E> {
    path: String,
    error: E,
}

impl<E: FromServerFnError> ServerFnUrlError<E> {
    /// Creates a new structure associating the server function at some path
    /// with a particular error.
    pub fn new(path: impl Display, error: E) -> Self {
        Self {
            path: path.to_string(),
            error,
        }
    }

    /// The error itself.
    pub fn error(&self) -> &E {
        &self.error
    }

    /// The path of the server function that generated this error.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Adds an encoded form of this server function error to the given base URL.
    pub fn to_url(&self, base: &str) -> Result<Url, url::ParseError> {
        let mut url = Url::parse(base)?;
        url.query_pairs_mut()
            .append_pair("__path", &self.path)
            .append_pair("__err", &URL_SAFE.encode(self.error.ser()));
        Ok(url)
    }

    /// Replaces any ServerFnUrlError info from the URL in the given string
    /// with the serialized success value given.
    pub fn strip_error_info(path: &mut String) {
        if let Ok(mut url) = Url::parse(&*path) {
            // NOTE: This is gross, but the Serializer you get from
            // .query_pairs_mut() isn't an Iterator so you can't just .retain().
            let pairs_previously = url
                .query_pairs()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<Vec<_>>();
            let mut pairs = url.query_pairs_mut();
            pairs.clear();
            for (key, value) in pairs_previously
                .into_iter()
                .filter(|(key, _)| key != "__path" && key != "__err")
            {
                pairs.append_pair(&key, &value);
            }
            drop(pairs);
            *path = url.to_string();
        }
    }

    /// Decodes an error from a URL.
    pub fn decode_err(err: &str) -> E {
        let decoded = match URL_SAFE.decode(err) {
            Ok(decoded) => decoded,
            Err(err) => {
                return ServerFnErrorErr::Deserialization(err.to_string())
                    .into_app_error();
            }
        };
        E::de(decoded.into())
    }
}

impl<E> From<ServerFnUrlError<E>> for ServerFnError<E> {
    fn from(error: ServerFnUrlError<E>) -> Self {
        error.error.into()
    }
}

impl<E> From<ServerFnUrlError<ServerFnError<E>>> for ServerFnError<E> {
    fn from(error: ServerFnUrlError<ServerFnError<E>>) -> Self {
        error.error
    }
}

#[derive(Debug, thiserror::Error)]
#[doc(hidden)]
/// Only used instantly only when a framework needs E: Error.
pub struct ServerFnErrorWrapper<E: FromServerFnError>(pub E);

impl<E: FromServerFnError> Display for ServerFnErrorWrapper<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            <E::Encoder as FormatType>::into_encoded_string(self.0.ser())
        )
    }
}

impl<E: FromServerFnError> FromStr for ServerFnErrorWrapper<E> {
    type Err = base64::DecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes =
            <E::Encoder as FormatType>::from_encoded_string(s).map_err(|e| {
                E::from_server_fn_error(ServerFnErrorErr::Deserialization(
                    e.to_string(),
                ))
            });
        let bytes = match bytes {
            Ok(bytes) => bytes,
            Err(err) => return Ok(Self(err)),
        };
        let err = E::de(bytes);
        Ok(Self(err))
    }
}

/// A trait for types that can be returned from a server function.
pub trait FromServerFnError: std::fmt::Debug + Sized + 'static {
    /// The encoding strategy used to serialize and deserialize this error type. Must implement the [`Encodes`](server_fn::Encodes) trait for references to the error type.
    type Encoder: Encodes<Self> + Decodes<Self>;

    /// Converts a [`ServerFnErrorErr`] into the application-specific custom error type.
    fn from_server_fn_error(value: ServerFnErrorErr) -> Self;

    /// Converts the custom error type to a [`String`].
    fn ser(&self) -> Bytes {
        Self::Encoder::encode(self).unwrap_or_else(|e| {
            Self::Encoder::encode(&Self::from_server_fn_error(
                ServerFnErrorErr::Serialization(e.to_string()),
            ))
            .expect(
                "error serializing should success at least with the \
                 Serialization error",
            )
        })
    }

    /// Deserializes the custom error type from a [`&str`].
    fn de(data: Bytes) -> Self {
        Self::Encoder::decode(data).unwrap_or_else(|e| {
            ServerFnErrorErr::Deserialization(e.to_string()).into_app_error()
        })
    }
}

/// A helper trait for converting a [`ServerFnErrorErr`] into an application-specific custom error type that implements [`FromServerFnError`].
pub trait IntoAppError<E> {
    /// Converts a [`ServerFnErrorErr`] into the application-specific custom error type.
    fn into_app_error(self) -> E;
}

impl<E> IntoAppError<E> for ServerFnErrorErr
where
    E: FromServerFnError,
{
    fn into_app_error(self) -> E {
        E::from_server_fn_error(self)
    }
}

#[doc(hidden)]
#[rustversion::attr(
    since(1.78),
    diagnostic::on_unimplemented(
        message = "{Self} is not a `Result` or aliased `Result`. Server \
                   functions must return a `Result` or aliased `Result`.",
        label = "Must return a `Result` or aliased `Result`.",
        note = "If you are trying to return an alias of `Result`, you must \
                also implement `FromServerFnError` for the error type."
    )
)]
/// A trait for extracting the error and ok types from a [`Result`]. This is used to allow alias types to be returned from server functions.
pub trait ServerFnMustReturnResult {
    /// The error type of the [`Result`].
    type Err;
    /// The ok type of the [`Result`].
    type Ok;
}

#[doc(hidden)]
impl<T, E> ServerFnMustReturnResult for Result<T, E> {
    type Err = E;
    type Ok = T;
}

#[test]
fn assert_from_server_fn_error_impl() {
    fn assert_impl<T: FromServerFnError>() {}

    assert_impl::<ServerFnError>();
}
