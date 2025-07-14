use std::{
    error::Error,
    fmt::{Debug, Display},
    str::FromStr,
};

use dioxus_lib::prelude::{dioxus_core::CapturedError, RenderError};
use serde::{de::DeserializeOwned, Serialize};
use server_fn::{
    codec::JsonEncoding,
    error::{FromServerFnError, ServerFnErrorErr},
};

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
pub type ServerFnResult<T = (), E = String> = std::result::Result<T, ServerFnError<E>>;

/// An error type for server functions. This may either be an error that occurred while running the server
/// function logic, or an error that occurred while communicating with the server inside the server function crate.
///
/// ## Usage
///
/// You can use the [`ServerFnError`] type in the Error type of your server function result or use the [`ServerFnResult`]
/// type as the return type of your server function. When you call the server function, you can handle the error directly
/// or convert it into a [`CapturedError`] to throw into the nearest [`ErrorBoundary`](dioxus_lib::prelude::ErrorBoundary).
///
/// ```rust
/// use dioxus::prelude::*;
///
/// #[server]
/// async fn parse_number(number: String) -> ServerFnResult<f32> {
///     // You can convert any error type into the `ServerFnError` with the `?` operator
///     let parsed_number: f32 = number.parse()?;
///     Ok(parsed_number)
/// }
///
/// #[component]
/// fn ParseNumberServer() -> Element {
///     let mut number = use_signal(|| "42".to_string());
///     let mut parsed = use_signal(|| None);
///
///     rsx! {
///         input {
///             value: "{number}",
///             oninput: move |e| number.set(e.value()),
///         }
///         button {
///             onclick: move |_| async move {
///                 // Call the server function to parse the number
///                 // If the result is Ok, continue running the closure, otherwise bubble up the
///                 // error to the nearest error boundary with `?`
///                 let result = parse_number(number()).await?;
///                 parsed.set(Some(result));
///                 Ok(())
///             },
///             "Parse Number"
///         }
///         if let Some(value) = parsed() {
///             p { "Parsed number: {value}" }
///         } else {
///             p { "No number parsed yet." }
///         }
///     }
/// }
/// ```
///
/// ## Differences from [`CapturedError`]
///
/// Both this error type and [`CapturedError`] can be used to represent boxed errors in dioxus. However, this error type
/// is more strict about the kinds of errors it can represent. [`CapturedError`] can represent any error that implements
/// the [`Error`] trait or can be converted to a string. [`CapturedError`] holds onto the type information of the error
/// and lets you downcast the error to its original type.
///
/// [`ServerFnError`] represents server function errors as [`String`]s by default without any additional type information.
/// This makes it easy to serialize the error to JSON and send it over the wire, but it means that you can't get the
/// original type information of the error back. If you need to preserve the type information of the error, you can use a
/// [custom error variant](#custom-error-variants) that holds onto the type information.
///
/// ## Custom error variants
///
/// The [`ServerFnError`] type accepts a generic type parameter `T` that is used to represent the error type used for server
/// functions. If you need to keep the type information of your error, you can create a custom error variant that implements
/// [`Serialize`] and [`DeserializeOwned`]. This allows you to serialize the error to JSON and send it over the wire,
/// while still preserving the type information.
///
/// ```rust
/// use dioxus::prelude::*;
/// use serde::{Deserialize, Serialize};
/// use std::fmt::Debug;
///
/// #[derive(Clone, Debug, Serialize, Deserialize)]
/// pub struct MyCustomError {
///     message: String,
///     code: u32,
/// }
///
/// impl MyCustomError {
///     pub fn new(message: String, code: u32) -> Self {
///         Self { message, code }
///     }
/// }
///
/// #[server]
/// async fn server_function() -> ServerFnResult<String, MyCustomError> {
///     // Return your custom error
///     Err(ServerFnError::ServerError(MyCustomError::new(
///         "An error occurred".to_string(),
///         404,
///     )))
/// }
/// ```
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum ServerFnError<T = String> {
    /// An error running the server function
    ServerError(T),

    /// An error communicating with the server
    CommunicationError(ServerFnErrorErr),
}

impl ServerFnError {
    /// Creates a new `ServerFnError` from something that implements `ToString`.
    ///
    /// # Examples
    /// ```rust
    /// use dioxus::prelude::*;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Serialize, Deserialize, Debug)]
    /// struct MyCustomError;
    /// impl std::fmt::Display for MyCustomError {
    ///    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///       write!(f, "My custom error")
    ///   }
    /// }
    ///
    /// #[server]
    /// async fn server_function() -> ServerFnResult<String, MyCustomError> {
    ///     // Return your custom error
    ///     Err(ServerFnError::new("Something went wrong"))
    /// }
    /// ```
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

    /// Returns a reference to the server error if it is a server error
    /// or `None` if it is a communication error.
    pub fn server_error(&self) -> Option<&T> {
        match self {
            ServerFnError::ServerError(err) => Some(err),
            ServerFnError::CommunicationError(_) => None,
        }
    }

    /// Returns a reference to the communication error if it is a communication error
    /// or `None` if it is a server error.
    pub fn communication_error(&self) -> Option<&ServerFnErrorErr> {
        match self {
            ServerFnError::ServerError(_) => None,
            ServerFnError::CommunicationError(err) => Some(err),
        }
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

impl From<ServerFnError> for RenderError {
    fn from(error: ServerFnError) -> Self {
        RenderError::Aborted(CapturedError::from(error))
    }
}

impl<E: Error> From<E> for ServerFnError {
    fn from(error: E) -> Self {
        Self::ServerError(error.to_string())
    }
}
