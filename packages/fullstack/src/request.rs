use bytes::Bytes;
use dioxus_fullstack_core::ServerFnError;
use http::HeaderMap;
use reqwest::{RequestBuilder, Response, StatusCode};
use std::{future::Future, pin::Pin};
use url::Url;

pub trait FromResponse<R = Response>: Sized {
    fn from_response(res: R) -> impl Future<Output = Result<Self, ServerFnError>> + Send;
}

pub trait ClientResponse {
    fn status(&self) -> StatusCode;
    fn headers(&self) -> &HeaderMap;
    fn url(&self) -> &Url;
    fn content_length(&self) -> Option<u64>;
    fn bytes(self) -> impl Future<Output = Result<Bytes, reqwest::Error>> + Send;
    fn byte_stream(self) -> impl futures_util::Stream<Item = Result<Bytes, reqwest::Error>>;
    fn original_request(&self);
}

pub trait IntoRequest<R = Response>: Sized {
    fn into_request(
        self,
        builder: RequestBuilder,
    ) -> impl Future<Output = Result<R, reqwest::Error>> + Send + 'static;
}

impl<A, R> IntoRequest<R> for (A,)
where
    A: IntoRequest<R> + 'static,
{
    fn into_request(
        self,
        builder: RequestBuilder,
    ) -> impl Future<Output = Result<R, reqwest::Error>> + Send + 'static {
        send_wrapper::SendWrapper::new(async move { A::into_request(self.0, builder).await })
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
    note = "`E` is either `From<ServerFnError> + Serialize`, `dioxus::Error` or `StatusCode`."
)]
pub trait AssertIsResult {}
impl<T, E> AssertIsResult for Result<T, E> {}

#[doc(hidden)]
pub fn assert_is_result<T: AssertIsResult>() {}

#[diagnostic::on_unimplemented(
    message = "The arguments to the server function must either be a single `impl FromRequest + IntoRequest` argument, or multiple `DeserializeOwned` arguments."
)]
pub trait AssertCanEncode {}

pub struct CantEncode;

pub struct EncodeIsVerified;
impl AssertCanEncode for EncodeIsVerified {}

#[doc(hidden)]
pub fn assert_can_encode(_t: impl AssertCanEncode) {}
