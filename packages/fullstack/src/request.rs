use dioxus_fullstack_core::{RequestError, ServerFnError};
#[cfg(feature = "server")]
use headers::Header;
use http::response::Parts;
use std::{future::Future, pin::Pin};

use crate::{ClientRequest, ClientResponse};

/// The `IntoRequest` trait allows types to be used as the body of a request to a HTTP endpoint or server function.
///
/// `IntoRequest` allows for types handle the calling of `ClientRequest::send` where the result is then
/// passed to `FromResponse` to decode the response.
///
/// You can think of the `IntoRequest` and `FromResponse` traits are "inverse" traits of the axum
/// `FromRequest` and `IntoResponse` traits. Just like a type can be decoded from a request via `FromRequest`,
/// a type can be encoded into a request via `IntoRequest`.
///
/// ## Generic State
///
/// `IntoRequest` is generic over the response type `R` which defaults to `ClientResponse`. The default
/// `ClientResponse` is the base response type that internally wraps `reqwest::Response`.
///
/// However, some responses might need state from the initial request to properly decode the response.
/// Most state can be extended via the `.extension()` method on `ClientRequest`. In some cases, like
/// websockets, the response needs to retain an initial connection from the request. Here, you can use
///  the `R` generic to specify a concrete response type. The resulting type that implements `FromResponse`
/// must also be generic over the same `R` type.
pub trait IntoRequest<R = ClientResponse>: Sized {
    fn into_request(
        self,
        req: ClientRequest,
    ) -> impl Future<Output = Result<R, RequestError>> + 'static;
}

impl<A, R> IntoRequest<R> for (A,)
where
    A: IntoRequest<R> + 'static + Send,
{
    fn into_request(
        self,
        req: ClientRequest,
    ) -> impl Future<Output = Result<R, RequestError>> + 'static {
        A::into_request(self.0, req)
    }
}

pub trait FromResponse<R = ClientResponse>: Sized {
    fn from_response(res: R) -> impl Future<Output = Result<Self, ServerFnError>>;
}

impl<A> FromResponse for A
where
    A: FromResponseParts,
{
    fn from_response(res: ClientResponse) -> impl Future<Output = Result<Self, ServerFnError>> {
        async move {
            let (parts, _body) = res.into_parts();
            let mut parts = parts;
            A::from_response_parts(&mut parts)
        }
    }
}

impl<A, B> FromResponse for (A, B)
where
    A: FromResponseParts,
    B: FromResponse,
{
    fn from_response(res: ClientResponse) -> impl Future<Output = Result<Self, ServerFnError>> {
        async move {
            let mut parts = res.make_parts();
            let a = A::from_response_parts(&mut parts)?;
            let b = B::from_response(res).await?;
            Ok((a, b))
        }
    }
}

impl<A, B, C> FromResponse for (A, B, C)
where
    A: FromResponseParts,
    B: FromResponseParts,
    C: FromResponse,
{
    fn from_response(res: ClientResponse) -> impl Future<Output = Result<Self, ServerFnError>> {
        async move {
            let mut parts = res.make_parts();
            let a = A::from_response_parts(&mut parts)?;
            let b = B::from_response_parts(&mut parts)?;
            let c = C::from_response(res).await?;
            Ok((a, b, c))
        }
    }
}

pub trait FromResponseParts
where
    Self: Sized,
{
    fn from_response_parts(parts: &mut Parts) -> Result<Self, ServerFnError>;
}

#[cfg(feature = "server")]
impl<T: Header> FromResponseParts for axum_extra::TypedHeader<T> {
    fn from_response_parts(parts: &mut Parts) -> Result<Self, ServerFnError> {
        use headers::HeaderMapExt;

        let t = parts
            .headers
            .typed_get::<T>()
            .ok_or_else(|| ServerFnError::Serialization("Invalid header value".into()))?;

        Ok(axum_extra::TypedHeader(t))
    }
}

/*
todo: make the serverfns return ServerFnRequest which lets us control the future better
*/
#[pin_project::pin_project]
#[must_use = "Requests do nothing unless you `.await` them"]
pub struct ServerFnRequest<Output> {
    _phantom: std::marker::PhantomData<Output>,
    #[pin]
    fut: Pin<Box<dyn Future<Output = Output> + Send>>,
}

impl<O> ServerFnRequest<O> {
    pub fn new(res: impl Future<Output = O> + Send + 'static) -> Self {
        ServerFnRequest {
            _phantom: std::marker::PhantomData,
            fut: Box::pin(res),
        }
    }
}

impl<T, E> std::future::Future for ServerFnRequest<Result<T, E>> {
    type Output = Result<T, E>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.project().fut.poll(cx)
    }
}

#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "The return type of a server function must be `Result<T, E>`",
    note = "`T` is either `impl IntoResponse` *or* `impl Serialize`",
    note = "`E` is either `From<ServerFnError> + Serialize`, `dioxus::CapturedError` or `StatusCode`."
)]
pub trait AssertIsResult {}
impl<T, E> AssertIsResult for Result<T, E> {}

#[doc(hidden)]
pub fn assert_is_result<T: AssertIsResult>() {}

#[diagnostic::on_unimplemented(message = r#"❌ Invalid Arguments to ServerFn ❌

The arguments to the server function must be either:

- a single `impl FromRequest + IntoRequest` argument
- or multiple `DeserializeOwned` arguments.

Did you forget to implement `IntoRequest` or `Deserialize` for one of the arguments?

`IntoRequest` is a trait that allows payloads to be sent to the server function.

> See https://dioxuslabs.com/learn/0.7/essentials/fullstack/server_functions for more details.

"#)]
pub trait AssertCanEncode {}

pub struct CantEncode;

pub struct EncodeIsVerified;
impl AssertCanEncode for EncodeIsVerified {}

#[diagnostic::on_unimplemented(message = r#"❌ Invalid return type from ServerFn ❌

The arguments to the server function must be either:

- a single `impl FromResponse` return type
- a single `impl Serialize + DeserializedOwned` return type

Did you forget to implement `FromResponse` or `DeserializeOwned` for one of the arguments?

`FromResponse` is a trait that allows payloads to be decoded from the server function response.

> See https://dioxuslabs.com/learn/0.7/essentials/fullstack/server_functions for more details.

"#)]
pub trait AssertCanDecode {}
pub struct CantDecode;
pub struct DecodeIsVerified;
impl AssertCanDecode for DecodeIsVerified {}

#[doc(hidden)]
pub fn assert_can_encode(_t: impl AssertCanEncode) {}

#[doc(hidden)]
pub fn assert_can_decode(_t: impl AssertCanDecode) {}
