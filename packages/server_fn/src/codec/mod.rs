//! The serialization/deserialization process for server functions consists of a series of steps,
//! each of which is represented by a different trait:
//! 1. [`IntoReq`]: The client serializes the [`ServerFn`] argument type into an HTTP request.
//! 2. The [`Client`] sends the request to the server.
//! 3. [`FromReq`]: The server deserializes the HTTP request back into the [`ServerFn`] type.
//! 4. The server calls [`ServerFn::run_body`] on the data.
//! 5. [`IntoRes`]: The server serializes the [`ServerFn::Output`] type into an HTTP response.
//! 6. The server integration applies any middleware from [`ServerFn::middlewares`] and responds to the request.
//! 7. [`FromRes`]: The client deserializes the response back into the [`ServerFn::Output`] type.
//!
//! Rather than a limited number of encodings, this crate allows you to define server functions that
//! mix and match the input encoding and output encoding. To define a new encoding, you simply implement
//! an input combination ([`IntoReq`] and [`FromReq`]) and/or an output encoding ([`IntoRes`] and [`FromRes`]).
//! This genuinely is an and/or: while some encodings can be used for both input and output (`Json`, `Cbor`, `Rkyv`),
//! others can only be used for input (`GetUrl`, `MultipartData`).

#[cfg(feature = "cbor")]
mod cbor;
#[cfg(feature = "cbor")]
pub use cbor::*;

mod json;
pub use json::*;

#[cfg(feature = "serde-lite")]
mod serde_lite;
#[cfg(feature = "serde-lite")]
pub use serde_lite::*;

#[cfg(feature = "rkyv")]
mod rkyv;
#[cfg(feature = "rkyv")]
pub use rkyv::*;

mod url;
pub use url::*;

#[cfg(feature = "multipart")]
mod multipart;
#[cfg(feature = "multipart")]
pub use multipart::*;

#[cfg(feature = "msgpack")]
mod msgpack;
#[cfg(feature = "msgpack")]
pub use msgpack::*;

#[cfg(feature = "postcard")]
mod postcard;
#[cfg(feature = "postcard")]
pub use postcard::*;

mod patch;
pub use patch::*;
mod post;
pub use post::*;
mod put;
pub use put::*;
mod stream;
use crate::ContentType;
use futures::Future;
use http::Method;
pub use stream::*;

/// Serializes a data type into an HTTP request, on the client.
///
/// Implementations use the methods of the [`ClientReq`](crate::request::ClientReq) trait to
/// convert data into a request body. They are often quite short, usually consisting
/// of just two steps:
/// 1. Serializing the data into some [`String`], [`Bytes`](bytes::Bytes), or [`Stream`](futures::Stream).
/// 2. Creating a request with a body of that type.
///
/// For example, here’s the implementation for [`Json`].
///
/// ```rust,ignore
/// impl<E, T, Request> IntoReq<Json, Request, E> for T
/// where
///     Request: ClientReq<E>,
///     T: Serialize + Send,
/// {
///     fn into_req(
///         self,
///         path: &str,
///         accepts: &str,
///     ) -> Result<Request, E> {
///         // try to serialize the data
///         let data = serde_json::to_string(&self)
///             .map_err(|e| ServerFnErrorErr::Serialization(e.to_string()).into_app_error())?;
///         // and use it as the body of a POST request
///         Request::try_new_post(path, accepts, Json::CONTENT_TYPE, data)
///     }
/// }
/// ```
pub trait IntoReq<Encoding, Request, E> {
    /// Attempts to serialize the arguments into an HTTP request.
    fn into_req(self, path: &str, accepts: &str) -> Result<Request, E>;
}

/// Deserializes an HTTP request into the data type, on the server.
///
/// Implementations use the methods of the [`Req`](crate::Req) trait to access whatever is
/// needed from the request. They are often quite short, usually consisting
/// of just two steps:
/// 1. Extracting the request body into some [`String`], [`Bytes`](bytes::Bytes), or [`Stream`](futures::Stream).
/// 2. Deserializing that data into the data type.
///
/// For example, here’s the implementation for [`Json`].
///
/// ```rust,ignore
/// impl<E, T, Request> FromReq<Json, Request, E> for T
/// where
///     // require the Request implement `Req`
///     Request: Req<E> + Send + 'static,
///     // require that the type can be deserialized with `serde`
///     T: DeserializeOwned,
///     E: FromServerFnError,
/// {
///     async fn from_req(
///         req: Request,
///     ) -> Result<Self, E> {
///         // try to convert the body of the request into a `String`
///         let string_data = req.try_into_string().await?;
///         // deserialize the data
///         serde_json::from_str(&string_data)
///             .map_err(|e| ServerFnErrorErr::Args(e.to_string()).into_app_error())
///     }
/// }
/// ```
pub trait FromReq<Encoding, Request, E>
where
    Self: Sized,
{
    /// Attempts to deserialize the arguments from a request.
    fn from_req(req: Request) -> impl Future<Output = Result<Self, E>> + Send;
}

/// Serializes the data type into an HTTP response.
///
/// Implementations use the methods of the [`Res`](crate::Res) trait to create a
/// response. They are often quite short, usually consisting
/// of just two steps:
/// 1. Serializing the data type to a [`String`], [`Bytes`](bytes::Bytes), or a [`Stream`](futures::Stream).
/// 2. Creating a response with that serialized value as its body.
///
/// For example, here’s the implementation for [`Json`].
///
/// ```rust,ignore
/// impl<E, T, Response> IntoRes<Json, Response, E> for T
/// where
///     Response: Res<E>,
///     T: Serialize + Send,
///     E: FromServerFnError,
/// {
///     async fn into_res(self) -> Result<Response, E> {
///         // try to serialize the data
///         let data = serde_json::to_string(&self)
///             .map_err(|e| ServerFnErrorErr::Serialization(e.to_string()).into())?;
///         // and use it as the body of a response
///         Response::try_from_string(Json::CONTENT_TYPE, data)
///     }
/// }
/// ```
pub trait IntoRes<Encoding, Response, E> {
    /// Attempts to serialize the output into an HTTP response.
    fn into_res(self) -> impl Future<Output = Result<Response, E>> + Send;
}

/// Deserializes the data type from an HTTP response.
///
/// Implementations use the methods of the [`ClientRes`](crate::ClientRes) trait to extract
/// data from a response. They are often quite short, usually consisting
/// of just two steps:
/// 1. Extracting a [`String`], [`Bytes`](bytes::Bytes), or a [`Stream`](futures::Stream)
///    from the response body.
/// 2. Deserializing the data type from that value.
///
/// For example, here’s the implementation for [`Json`].
///
/// ```rust,ignore
/// impl<E, T, Response> FromRes<Json, Response, E> for T
/// where
///     Response: ClientRes<E> + Send,
///     T: DeserializeOwned + Send,
///     E: FromServerFnError,
/// {
///     async fn from_res(
///         res: Response,
///     ) -> Result<Self, E> {
///         // extracts the request body
///         let data = res.try_into_string().await?;
///         // and tries to deserialize it as JSON
///         serde_json::from_str(&data)
///             .map_err(|e| ServerFnErrorErr::Deserialization(e.to_string()).into_app_error())
///     }
/// }
/// ```
pub trait FromRes<Encoding, Response, E>
where
    Self: Sized,
{
    /// Attempts to deserialize the outputs from a response.
    fn from_res(res: Response) -> impl Future<Output = Result<Self, E>> + Send;
}

/// Defines a particular encoding format, which can be used for serializing or deserializing data.
pub trait Encoding: ContentType {
    /// The HTTP method used for requests.
    ///
    /// This should be `POST` in most cases.
    const METHOD: Method;
}
