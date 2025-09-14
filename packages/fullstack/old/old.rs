// /// Defines a function that runs only on the server, but can be called from the server or the client.
// ///
// /// The type for which `ServerFn` is implemented is actually the type of the arguments to the function,
// /// while the function body itself is implemented in [`run_body`](ServerFn::run_body).
// ///
// /// This means that `Self` here is usually a struct, in which each field is an argument to the function.
// /// In other words,
// /// ```rust,ignore
// /// #[server]
// /// pub async fn my_function(foo: String, bar: usize) -> Result<usize, ServerFnError> {
// ///     Ok(foo.len() + bar)
// /// }
// /// ```
// /// should expand to
// /// ```rust,ignore
// /// #[derive(Serialize, Deserialize)]
// /// pub struct MyFunction {
// ///     foo: String,
// ///     bar: usize
// /// }
// ///
// /// impl ServerFn for MyFunction {
// ///     async fn run_body() -> Result<usize, ServerFnError> {
// ///         Ok(foo.len() + bar)
// ///     }
// ///
// ///     // etc.
// /// }
// /// ```
// pub trait ServerFn: Send + Sized {
//     /// A unique path for the server function’s API endpoint, relative to the host, including its prefix.
//     const PATH: &'static str;

//     /// The HTTP method used for requests.
//     const METHOD: Method;

//     // /// The protocol the server function uses to communicate with the client.
//     // type Protocol: Protocol<Self, Self::Output>;

//     /// The return type of the server function.
//     ///
//     /// This needs to be converted into `ServerResponse` on the server side, and converted
//     /// *from* `ClientResponse` when received by the client.
//     type Output: Send;

//     // /// The type of error in the server function return.
//     // /// Typically [`ServerFnError`], but allowed to be any type that implements [`FromServerFnError`].
//     // type Error: FromServerFnError + Send + Sync;

//     // /// The type of error in the server function for stream items sent from the client to the server.
//     // /// Typically [`ServerFnError`], but allowed to be any type that implements [`FromServerFnError`].
//     // type InputStreamError: FromServerFnError + Send + Sync;

//     // /// The type of error in the server function for stream items sent from the server to the client.
//     // /// Typically [`ServerFnError`], but allowed to be any type that implements [`FromServerFnError`].
//     // type OutputStreamError: FromServerFnError + Send + Sync;

//     /// Returns [`Self::PATH`].
//     fn url() -> &'static str {
//         Self::PATH
//     }

//     /// Middleware that should be applied to this server function.
//     fn middlewares() -> Vec<Arc<dyn Layer<HybridRequest, HybridResponse>>> {
//         // ) -> Vec<Arc<dyn Layer<ServerFnServerRequest<Self>, ServerFnServerResponse<Self>>>> {
//         Vec::new()
//     }

//     /// The body of the server function. This will only run on the server.
//     fn run_body(self) -> impl Future<Output = Result<Self::Output, HybridError>> + Send;
//     // fn run_body(self) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send;

//     fn form_responder() -> bool {
//         false
//     }

//     #[doc(hidden)]
//     fn run_on_server(
//         req: HybridRequest,
//         // req: ServerFnServerRequest<Self>,
//     ) -> impl Future<Output = HybridResponse> + Send {
//         // ) -> impl Future<Output = ServerFnServerResponse<Self>> + Send {
//         // Server functions can either be called by a real Client,
//         // or directly by an HTML <form>. If they're accessed by a <form>, default to
//         // redirecting back to the Referer.
//         // #[cfg(feature = "form-redirects")]
//         // let accepts_html = req
//         //     .accepts()
//         //     .map(|n| n.contains("text/html"))
//         //     .unwrap_or(false);

//         // #[cfg(feature = "form-redirects")]
//         // let mut referer = req.referer().as_deref().map(ToOwned::to_owned);

//         async move {
//             // #[allow(unused_variables, unused_mut)]
//             // used in form redirects feature
//             // let (mut res, err) = Self::Protocol::run_server(req, Self::run_body)
//             // let (mut res, err) = Self::Protocol::run_server(req, Self::run_body)
//             //     .await
//             //     .map(|res| (res, None as Option<HybridError>))
//             //     .unwrap_or_else(|e| {
//             //         todo!()
//             //         // (
//             //         //     <<Self as ServerFn>::Server as Server<
//             //         //         Self::Error,
//             //         //         Self::InputStreamError,
//             //         //         Self::OutputStreamError,
//             //         //     >>::Response::error_response(Self::PATH, e.ser()),
//             //         //     Some(e),
//             //         // )
//             //     });

//             // // if it accepts HTML, we'll redirect to the Referer
//             // #[cfg(feature = "form-redirects")]
//             // if accepts_html {
//             //     // if it had an error, encode that error in the URL
//             //     if let Some(err) = err {
//             //         if let Ok(url) = ServerFnUrlError::new(Self::PATH, err)
//             //             .to_url(referer.as_deref().unwrap_or("/"))
//             //         {
//             //             referer = Some(url.to_string());
//             //         }
//             //     }
//             //     // otherwise, strip error info from referer URL, as that means it's from a previous
//             //     // call
//             //     else if let Some(referer) = referer.as_mut() {
//             //         ServerFnUrlError::<Self::Error>::strip_error_info(referer)
//             //     }

//             //     // set the status code and Location header
//             //     res.redirect(referer.as_deref().unwrap_or("/"));
//             // }
//             // res
//             todo!()
//         }
//     }

//     #[doc(hidden)]
//     async fn run_on_client(self) -> Result<Self::Output, HybridError> {
//         // fn run_on_client(self) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send {
//         // Self::Protocol::run_client(Self::PATH, self).await
//         todo!()
//     }
// }

// Error = HybridError,
// InputStreamError = Error,
// OutputStreamError = Error,

// /// A client defines a pair of request/response types and the logic to send
// /// and receive them.
// ///
// /// This trait is implemented for things like a browser `fetch` request or for
// /// the `reqwest` trait. It should almost never be necessary to implement it
// /// yourself, unless you’re trying to use an alternative HTTP crate on the client side.
// pub trait Client<Error = HybridError, InputStreamError = Error, OutputStreamError = Error> {
//     /// Sends the request and receives a response.
//     fn send(req: HybridRequest) -> impl Future<Output = Result<HybridResponse, Error>> + Send;

//     /// Opens a websocket connection to the server.
//     #[allow(clippy::type_complexity)]
//     fn open_websocket(
//         path: &str,
//     ) -> impl Future<
//         Output = Result<
//             (
//                 impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static,
//                 impl Sink<Bytes> + Send + 'static,
//             ),
//             Error,
//         >,
//     > + Send;
// }

// pub type ServerFnResult<T = (), E = String> = std::result::Result<T, ServerFnError<E>>;

// /// An error type for server functions. This may either be an error that occurred while running the server
// /// function logic, or an error that occurred while communicating with the server inside the server function crate.
// ///
// /// ## Usage
// ///
// /// You can use the [`ServerFnError`] type in the Error type of your server function result or use the [`ServerFnResult`]
// /// type as the return type of your server function. When you call the server function, you can handle the error directly
// /// or convert it into a [`CapturedError`] to throw into the nearest [`ErrorBoundary`](dioxus_core::ErrorBoundary).
// ///
// /// ```rust
// /// use dioxus::prelude::*;
// ///
// /// #[server]
// /// async fn parse_number(number: String) -> ServerFnResult<f32> {
// ///     // You can convert any error type into the `ServerFnError` with the `?` operator
// ///     let parsed_number: f32 = number.parse()?;
// ///     Ok(parsed_number)
// /// }
// ///
// /// #[component]
// /// fn ParseNumberServer() -> Element {
// ///     let mut number = use_signal(|| "42".to_string());
// ///     let mut parsed = use_signal(|| None);
// ///
// ///     rsx! {
// ///         input {
// ///             value: "{number}",
// ///             oninput: move |e| number.set(e.value()),
// ///         }
// ///         button {
// ///             onclick: move |_| async move {
// ///                 // Call the server function to parse the number
// ///                 // If the result is Ok, continue running the closure, otherwise bubble up the
// ///                 // error to the nearest error boundary with `?`
// ///                 let result = parse_number(number()).await?;
// ///                 parsed.set(Some(result));
// ///                 Ok(())
// ///             },
// ///             "Parse Number"
// ///         }
// ///         if let Some(value) = parsed() {
// ///             p { "Parsed number: {value}" }
// ///         } else {
// ///             p { "No number parsed yet." }
// ///         }
// ///     }
// /// }
// /// ```
// ///
// /// ## Differences from [`CapturedError`]
// ///
// /// Both this error type and [`CapturedError`] can be used to represent boxed errors in dioxus. However, this error type
// /// is more strict about the kinds of errors it can represent. [`CapturedError`] can represent any error that implements
// /// the [`Error`] trait or can be converted to a string. [`CapturedError`] holds onto the type information of the error
// /// and lets you downcast the error to its original type.
// ///
// /// [`ServerFnError`] represents server function errors as [`String`]s by default without any additional type information.
// /// This makes it easy to serialize the error to JSON and send it over the wire, but it means that you can't get the
// /// original type information of the error back. If you need to preserve the type information of the error, you can use a
// /// [custom error variant](#custom-error-variants) that holds onto the type information.
// ///
// /// ## Custom error variants
// ///
// /// The [`ServerFnError`] type accepts a generic type parameter `T` that is used to represent the error type used for server
// /// functions. If you need to keep the type information of your error, you can create a custom error variant that implements
// /// [`Serialize`] and [`DeserializeOwned`]. This allows you to serialize the error to JSON and send it over the wire,
// /// while still preserving the type information.
// ///
// /// ```rust
// /// use dioxus::prelude::*;
// /// use serde::{Deserialize, Serialize};
// /// use std::fmt::Debug;
// ///
// /// #[derive(Clone, Debug, Serialize, Deserialize)]
// /// pub struct MyCustomError {
// ///     message: String,
// ///     code: u32,
// /// }
// ///
// /// impl MyCustomError {
// ///     pub fn new(message: String, code: u32) -> Self {
// ///         Self { message, code }
// ///     }
// /// }
// ///
// /// #[server]
// /// async fn server_function() -> ServerFnResult<String, MyCustomError> {
// ///     // Return your custom error
// ///     Err(ServerFnError::ServerError(MyCustomError::new(
// ///         "An error occurred".to_string(),
// ///         404,
// ///     )))
// /// }
// /// ```
// #[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
// pub enum ServerFnError<T = String> {
//     /// An error running the server function
//     ServerError(T),

//     /// An error communicating with the server
//     CommunicationError(ServerFnError),
// }

// impl ServerFnError {
//     /// Creates a new `ServerFnError` from something that implements `ToString`.
//     ///
//     /// # Examples
//     /// ```rust
//     /// use dioxus::prelude::*;
//     /// use serde::{Serialize, Deserialize};
//     ///
//     /// #[server]
//     /// async fn server_function() -> ServerFnResult<String> {
//     ///     // Return your custom error
//     ///     Err(ServerFnError::new("Something went wrong"))
//     /// }
//     /// ```
//     pub fn new(error: impl ToString) -> Self {
//         Self::ServerError(error.to_string())
//     }
// }

// impl From<ServerFnError> for CapturedError {
//     fn from(error: ServerFnError) -> Self {
//         Self::from_display(error)
//     }
// }

// impl From<ServerFnError> for RenderError {
//     fn from(error: ServerFnError) -> Self {
//         RenderError::Aborted(CapturedError::from(error))
//     }
// }

// impl<E: std::error::Error> Into<E> for ServerFnError {
//     fn into(self) -> E {
//         todo!()
//     }
// }
// impl<E: std::error::Error> From<E> for ServerFnError {
//     fn from(error: E) -> Self {
//         Self::ServerError(error.to_string())
//     }
// }

// impl Into<RenderError> for ServerFnError {
//     fn into(self) -> RenderError {
//         todo!()
//     }
// }

// impl<T: Serialize + DeserializeOwned + std::fmt::Debug + 'static> FromServerFnError
//     for ServerFnError<T>
// {
//     type Encoder = crate::codec::JsonEncoding;

//     fn from_server_fn_error(err: ServerFnError) -> Self {
//         Self::CommunicationError(err)
//     }
// }

// impl<T: FromStr> FromStr for ServerFnError<T> {
//     type Err = <T as FromStr>::Err;

//     fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
//         std::result::Result::Ok(Self::ServerError(T::from_str(s)?))
//     }
// }

// impl<T: Display> Display for ServerFnError<T> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             ServerFnError::ServerError(err) => write!(f, "Server error: {err}"),
//             ServerFnError::CommunicationError(err) => write!(f, "Communication error: {err}"),
//         }
//     }
// }

// #[cfg(feature = "axum-no-default")]
// mod axum {
//     use super::{BoxedService, Service};
//     use crate::error::ServerFnError;
//     use axum::body::Body;
//     use bytes::Bytes;
//     use http::{Request, Response};
//     use std::{future::Future, pin::Pin};

//     impl<S> super::Service<Request<Body>, Response<Body>> for S
//     where
//         S: tower::Service<Request<Body>, Response = Response<Body>>,
//         S::Future: Send + 'static,
//         S::Error: std::fmt::Display + Send + 'static,
//     {
//         fn run(
//             &mut self,
//             req: Request<Body>,
//             ser: fn(ServerFnError) -> Bytes,
//         ) -> Pin<Box<dyn Future<Output = Response<Body>> + Send>> {
//             let path = req.uri().path().to_string();
//             let inner = self.call(req);
//             todo!()
//             // Box::pin(async move {
//             //     inner.await.unwrap_or_else(|e| {
//             //         let err = ser(ServerFnError::MiddlewareError(e.to_string()));
//             //         Response::<Body>::error_response(&path, err)
//             //     })
//             // })
//         }
//     }

//     impl tower::Service<Request<Body>> for BoxedService<Request<Body>, Response<Body>> {
//         type Response = Response<Body>;
//         type Error = ServerFnError;
//         type Future =
//             Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

//         fn poll_ready(
//             &mut self,
//             _cx: &mut std::task::Context<'_>,
//         ) -> std::task::Poll<Result<(), Self::Error>> {
//             Ok(()).into()
//         }

//         fn call(&mut self, req: Request<Body>) -> Self::Future {
//             let inner = self.service.run(req, self.ser);
//             Box::pin(async move { Ok(inner.await) })
//         }
//     }

//     impl<L> super::Layer<Request<Body>, Response<Body>> for L
//     where
//         L: tower_layer::Layer<BoxedService<Request<Body>, Response<Body>>> + Sync + Send + 'static,
//         L::Service: Service<Request<Body>, Response<Body>> + Send + 'static,
//     {
//         fn layer(
//             &self,
//             inner: BoxedService<Request<Body>, Response<Body>>,
//         ) -> BoxedService<Request<Body>, Response<Body>> {
//             BoxedService::new(inner.ser, self.layer(inner))
//         }
//     }
// }

// impl From<ServerFnError> for Error {
//     fn from(e: ServerFnError) -> Self {
//         Error::from(ServerFnErrorWrapper(e))
//     }
// }

// /// An empty value indicating that there is no custom error type associated
// /// with this server function.
// #[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Hash, PartialOrd, Ord, Clone, Copy)]
// // #[cfg_attr(
// //     feature = "rkyv",
// //     derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
// // )]
// #[deprecated(
//     since = "0.8.0",
//     note = "Now server_fn can return any error type other than ServerFnError, \
//             so the WrappedServerError variant will be removed in 0.9.0"
// )]
// pub struct NoCustomError;

// // Implement `Display` for `NoCustomError`
// impl fmt::Display for NoCustomError {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "Unit Type Displayed")
//     }
// }

// impl FromStr for NoCustomError {
//     type Err = ();

//     fn from_str(_s: &str) -> Result<Self, Self::Err> {
//         Ok(NoCustomError)
//     }
// }

// /// Wraps some error type, which may implement any of [`Error`](trait@std::error::Error), [`Clone`], or
// /// [`Display`].
// #[derive(Debug)]
// #[deprecated(
//     since = "0.8.0",
//     note = "Now server_fn can return any error type other than ServerFnError, \
//             so the WrappedServerError variant will be removed in 0.9.0"
// )]
// pub struct WrapError<T>(pub T);

// /// A helper macro to convert a variety of different types into `ServerFnError`.
// /// This should mostly be used if you are implementing `From<ServerFnError>` for `YourError`.
// #[macro_export]
// #[deprecated(
//     since = "0.8.0",
//     note = "Now server_fn can return any error type other than ServerFnError, \
//             so the WrappedServerError variant will be removed in 0.9.0"
// )]
// macro_rules! server_fn_error {
//     () => {{
//         use $crate::{ViaError, WrapError};
//         (&&&&&WrapError(())).to_server_error()
//     }};
//     ($err:expr) => {{
//         use $crate::error::{ViaError, WrapError};
//         match $err {
//             error => (&&&&&WrapError(error)).to_server_error(),
//         }
//     }};
// }

// /// This trait serves as the conversion method between a variety of types
// /// and [`ServerFnError`].
// #[deprecated(
//     since = "0.8.0",
//     note = "Now server_fn can return any error type other than ServerFnError, \
//             so users should place their custom error type instead of \
//             ServerFnError"
// )]
// pub trait ViaError<E> {
//     /// Converts something into an error.
//     fn to_server_error(&self) -> ServerFnError;
// }

// // This impl should catch if you fed it a [`ServerFnError`] already.
// impl<E: ServerFnErrorKind + std::error::Error + Clone> ViaError<E>
//     for &&&&WrapError<ServerFnError>
// {
//     fn to_server_error(&self) -> ServerFnError {
//         self.0.clone()
//     }
// }

// // A type tag for ServerFnError so we can special case it
// #[deprecated]
// pub(crate) trait ServerFnErrorKind {}

// impl ServerFnErrorKind for ServerFnError {}

// // This impl should catch passing () or nothing to server_fn_error
// impl ViaError<NoCustomError> for &&&WrapError<()> {
//     fn to_server_error(&self) -> ServerFnError {
//         ServerFnError::WrappedServerError(NoCustomError)
//     }
// }

// // This impl will catch any type that implements any type that impls
// // Error and Clone, so that it can be wrapped into ServerFnError
// impl<E: std::error::Error + Clone> ViaError<E> for &&WrapError<E> {
//     fn to_server_error(&self) -> ServerFnError {
//         ServerFnError::WrappedServerError(self.0.clone())
//     }
// }

// // If it doesn't impl Error, but does impl Display and Clone,
// // we can still wrap it in String form
// impl<E: Display + Clone> ViaError<E> for &WrapError<E> {
//     fn to_server_error(&self) -> ServerFnError {
//         ServerFnError::ServerError(self.0.to_string())
//     }
// }

// // This is what happens if someone tries to pass in something that does
// // not meet the above criteria
// impl<E> ViaError<E> for WrapError<E> {
//     #[track_caller]
//     fn to_server_error(&self) -> ServerFnError {
//         panic!(
//             "At {}, you call `to_server_error()` or use  `server_fn_error!` \
//              with a value that does not implement `Clone` and either `Error` \
//              or `Display`.",
//             std::panic::Location::caller()
//         );
//     }
// }

// pub struct RequestBuilder {}
// impl RequestBuilder {}
// /// Opens a websocket connection to the server.
// #[allow(clippy::type_complexity)]
// pub fn open_websocket(
//     path: &str,
// ) -> impl Future<
//     Output = Result<
//         (
//             impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static,
//             impl Sink<Bytes> + Send + 'static,
//         ),
//         HybridError,
//     >,
// > + Send {
//     async {
//         Ok((
//             async move { todo!() }.into_stream(),
//             async move { todo!() }.into_stream(),
//         ))
//     }
// }
// #[cfg(feature = "browser")]
// /// Implements [`Client`] for a `fetch` request in the browser.
// pub mod browser {
//     use super::Client;
//     use crate::{
//         error::{FromServerFnError, IntoAppError, ServerFnError},
//         request::browser::{BrowserRequest, RequestInner},
//         response::browser::BrowserResponse,
//     };
//     use bytes::Bytes;
//     use futures::{Sink, SinkExt, StreamExt};
//     use gloo_net::websocket::{Message, WebSocketError};
//     use send_wrapper::SendWrapper;
//     use std::future::Future;

//     /// Implements [`Client`] for a `fetch` request in the browser.
//     pub struct BrowserClient;

//     impl<
//             Error: FromServerFnError,
//             InputStreamError: FromServerFnError,
//             OutputStreamError: FromServerFnError,
//         > Client<Error, InputStreamError, OutputStreamError> for BrowserClient
//     {
//         type Request = BrowserRequest;
//         type Response = BrowserResponse;

//         fn send(
//             req: Self::Request,
//         ) -> impl Future<Output = Result<Self::Response, Error>> + Send
//         {
//             SendWrapper::new(async move {
//                 let req = req.0.take();
//                 let RequestInner {
//                     request,
//                     mut abort_ctrl,
//                 } = req;
//                 let res = request
//                     .send()
//                     .await
//                     .map(|res| BrowserResponse(SendWrapper::new(res)))
//                     .map_err(|e| {
//                         ServerFnError::Request(e.to_string())
//                             .into_app_error()
//                     });

//                 // at this point, the future has successfully resolved without being dropped, so we
//                 // can prevent the `AbortController` from firing
//                 if let Some(ctrl) = abort_ctrl.as_mut() {
//                     ctrl.prevent_cancellation();
//                 }
//                 res
//             })
//         }

//         fn open_websocket(
//             url: &str,
//         ) -> impl Future<
//             Output = Result<
//                 (
//                     impl futures::Stream<Item = Result<Bytes, Bytes>>
//                         + Send
//                         + 'static,
//                     impl futures::Sink<Bytes> + Send + 'static,
//                 ),
//                 Error,
//             >,
//         > + Send {
//             SendWrapper::new(async move {
//                 let websocket =
//                     gloo_net::websocket::futures::WebSocket::open(url)
//                         .map_err(|err| {
//                             web_sys::console::error_1(&err.to_string().into());
//                             Error::from_server_fn_error(
//                                 ServerFnError::Request(err.to_string()),
//                             )
//                         })?;
//                 let (sink, stream) = websocket.split();

//                 let stream = stream.map(|message| match message {
//                     Ok(message) => Ok(match message {
//                         Message::Text(text) => Bytes::from(text),
//                         Message::Bytes(bytes) => Bytes::from(bytes),
//                     }),
//                     Err(err) => {
//                         web_sys::console::error_1(&err.to_string().into());
//                         Err(OutputStreamError::from_server_fn_error(
//                             ServerFnError::Request(err.to_string()),
//                         )
//                         .ser())
//                     }
//                 });
//                 let stream = SendWrapper::new(stream);

//                 struct SendWrapperSink<S> {
//                     sink: SendWrapper<S>,
//                 }

//                 impl<S> SendWrapperSink<S> {
//                     fn new(sink: S) -> Self {
//                         Self {
//                             sink: SendWrapper::new(sink),
//                         }
//                     }
//                 }

//                 impl<S, Item> Sink<Item> for SendWrapperSink<S>
//                 where
//                     S: Sink<Item> + Unpin,
//                 {
//                     type Error = S::Error;

//                     fn poll_ready(
//                         self: std::pin::Pin<&mut Self>,
//                         cx: &mut std::task::Context<'_>,
//                     ) -> std::task::Poll<Result<(), Self::Error>>
//                     {
//                         self.get_mut().sink.poll_ready_unpin(cx)
//                     }

//                     fn start_send(
//                         self: std::pin::Pin<&mut Self>,
//                         item: Item,
//                     ) -> Result<(), Self::Error> {
//                         self.get_mut().sink.start_send_unpin(item)
//                     }

//                     fn poll_flush(
//                         self: std::pin::Pin<&mut Self>,
//                         cx: &mut std::task::Context<'_>,
//                     ) -> std::task::Poll<Result<(), Self::Error>>
//                     {
//                         self.get_mut().sink.poll_flush_unpin(cx)
//                     }

//                     fn poll_close(
//                         self: std::pin::Pin<&mut Self>,
//                         cx: &mut std::task::Context<'_>,
//                     ) -> std::task::Poll<Result<(), Self::Error>>
//                     {
//                         self.get_mut().sink.poll_close_unpin(cx)
//                     }
//                 }

//                 let sink = sink.with(|message: Bytes| async move {
//                     Ok::<Message, WebSocketError>(Message::Bytes(
//                         message.into(),
//                     ))
//                 });
//                 let sink = SendWrapperSink::new(Box::pin(sink));

//                 Ok((stream, sink))
//             })
//         }

//         fn spawn(future: impl Future<Output = ()> + Send + 'static) {
//             wasm_bindgen_futures::spawn_local(future);
//         }
//     }
// }

// // #[cfg(feature = "reqwest")]
// /// Implements [`Client`] for a request made by [`reqwest`].
// pub mod reqwest {
//     use super::{get_server_url, Client};
//     use crate::{
//         error::{FromServerFnError, IntoAppError, ServerFnError},
//         request::reqwest::CLIENT,
//         HybridRequest, HybridResponse,
//     };
//     use bytes::Bytes;
//     use futures::{SinkExt, StreamExt, TryFutureExt};
//     use reqwest::{Request, Response};
//     use std::future::Future;

//     /// Implements [`Client`] for a request made by [`reqwest`].
//     pub struct ReqwestClient;

//     impl<
//             Error: FromServerFnError,
//             InputStreamError: FromServerFnError,
//             OutputStreamError: FromServerFnError,
//         > Client<Error, InputStreamError, OutputStreamError> for ReqwestClient
//     {
//         fn send(req: HybridRequest) -> impl Future<Output = Result<HybridResponse, Error>> + Send {
//             // CLIENT
//             //     .execute(req)
//             //     .map_err(|e| ServerFnError::Request(e.to_string()).into_app_error())

//             async { Ok(HybridResponse {}) }
//         }

//         async fn open_websocket(
//             path: &str,
//         ) -> Result<
//             (
//                 impl futures::Stream<Item = Result<Bytes, Bytes>> + Send + 'static,
//                 impl futures::Sink<Bytes> + Send + 'static,
//             ),
//             Error,
//         > {
//             let mut websocket_server_url = get_server_url().to_string();
//             if let Some(postfix) = websocket_server_url.strip_prefix("http://") {
//                 websocket_server_url = format!("ws://{postfix}");
//             } else if let Some(postfix) = websocket_server_url.strip_prefix("https://") {
//                 websocket_server_url = format!("wss://{postfix}");
//             }
//             let url = format!("{websocket_server_url}{path}");
//             let (ws_stream, _) = tokio_tungstenite::connect_async(url)
//                 .await
//                 .map_err(|e| Error::from_server_fn_error(ServerFnError::Request(e.to_string())))?;

//             let (write, read) = ws_stream.split();

//             Ok((
//                 read.map(|msg| match msg {
//                     Ok(msg) => Ok(msg.into_data()),
//                     Err(e) => Err(
//                         OutputStreamError::from_server_fn_error(ServerFnError::Request(
//                             e.to_string(),
//                         ))
//                         .ser(),
//                     ),
//                 }),
//                 write.with(|msg: Bytes| async move {
//                     Ok::<
//                         tokio_tungstenite::tungstenite::Message,
//                         tokio_tungstenite::tungstenite::Error,
//                     >(tokio_tungstenite::tungstenite::Message::Binary(msg))
//                 }),
//             ))
//         }
//     }
// }

// /// The protocol that a server function uses to communicate with the client. This trait handles
// /// the server and client side of running a server function. It is implemented for the [`Http`] and
// /// [`Websocket`] protocols and can be used to implement custom protocols.
// pub trait Protocol<Input, Output> {
//     /// The HTTP method used for requests.
//     const METHOD: Method;

//     /// Run the server function on the server. The implementation should handle deserializing the
//     /// input, running the server function, and serializing the output.
//     fn run_server<F, Fut>(
//         request: HybridRequest,
//         server_fn: F,
//     ) -> impl Future<Output = Result<HybridResponse, HybridError>> + Send
//     where
//         F: Fn(Input) -> Fut + Send,
//         Fut: Future<Output = Result<Output, HybridError>> + Send;

//     /// Run the server function on the client. The implementation should handle serializing the
//     /// input, sending the request, and deserializing the output.
//     fn run_client(
//         path: &str,
//         input: Input,
//     ) -> impl Future<Output = Result<Output, HybridError>> + Send;
// }

// /// The http protocol with specific input and output encodings for the request and response. This is
// /// the default protocol server functions use if no override is set in the server function macro
// ///
// /// The http protocol accepts two generic argument that define how the input and output for a server
// /// function are turned into HTTP requests and responses. For example, [`Http<GetUrl, Json>`] will
// /// accept a Url encoded Get request and return a JSON post response.
// ///
// /// # Example
// ///
// /// ```rust, no_run
// /// # use server_fn_macro_default::server;
// /// use serde::{Serialize, Deserialize};
// /// use server_fn::{Http, ServerFnError, codec::{Json, GetUrl}};
// ///
// /// #[derive(Debug, Clone, Serialize, Deserialize)]
// /// pub struct Message {
// ///     user: String,
// ///     message: String,
// /// }
// ///
// /// // The http protocol can be used on any server function that accepts and returns arguments that implement
// /// // the [`IntoReq`] and [`FromRes`] traits.
// /// //
// /// // In this case, the input and output encodings are [`GetUrl`] and [`Json`], respectively which requires
// /// // the items to implement [`IntoReq<GetUrl, ...>`] and [`FromRes<Json, ...>`]. Both of those implementations
// /// // require the items to implement [`Serialize`] and [`Deserialize`].
// /// # #[cfg(feature = "browser")] {
// /// #[server(protocol = Http<GetUrl, Json>)]
// /// async fn echo_http(
// ///     input: Message,
// /// ) -> Result<Message, ServerFnError> {
// ///     Ok(input)
// /// }
// /// # }
// /// ```
// pub struct Http<InputProtocol, OutputProtocol>(PhantomData<(InputProtocol, OutputProtocol)>);

// impl<InputProtocol, OutputProtocol, Input, Output> Protocol<Input, Output>
//     for Http<InputProtocol, OutputProtocol>
// where
//     Input: FromReq<InputProtocol> + IntoReq<InputProtocol> + Send,
//     Output: IntoRes<OutputProtocol> + FromRes<OutputProtocol> + Send,
//     InputProtocol: Encoding,
//     OutputProtocol: Encoding,
// {
//     const METHOD: Method = InputProtocol::METHOD;

//     fn run_server<F, Fut>(
//         request: HybridRequest,
//         server_fn: F,
//     ) -> impl Future<Output = Result<HybridResponse, HybridError>> + Send
//     where
//         F: Fn(Input) -> Fut + Send,
//         Fut: Future<Output = Result<Output, HybridError>> + Send,
//     {
//         async move {
//             let input = Input::from_req(request).await?;

//             let output = server_fn(input).await?;

//             let response = Output::into_res(output).await?;

//             Ok(response)
//         }
//     }

//     fn run_client(
//         path: &str,
//         input: Input,
//     ) -> impl Future<Output = Result<Output, HybridError>> + Send {
//         async move {
//             // create and send request on client
//             let req = input.into_req(path, OutputProtocol::CONTENT_TYPE)?;
//             let res: HybridResponse = crate::client::current::send(req).await?;

//             let status = res.status();
//             let location = res.location();
//             let has_redirect_header = res.has_redirect();

//             // if it returns an error status, deserialize the error using the error's decoder.
//             let res = if (400..=599).contains(&status) {
//                 Err(HybridError::de(res.try_into_bytes().await?))
//             } else {
//                 // otherwise, deserialize the body as is
//                 let output = Output::from_res(res).await?;
//                 Ok(output)
//             }?;

//             // if redirected, call the redirect hook (if that's been set)
//             if (300..=399).contains(&status) || has_redirect_header {
//                 call_redirect_hook(&location);
//             }

//             Ok(res)
//         }
//     }
// }

// /// The websocket protocol that encodes the input and output streams using a websocket connection.
// ///
// /// The websocket protocol accepts two generic argument that define the input and output serialization
// /// formats. For example, [`Websocket<CborEncoding, JsonEncoding>`] would accept a stream of Cbor-encoded messages
// /// and return a stream of JSON-encoded messages.
// ///
// /// # Example
// ///
// /// ```rust, no_run
// /// # use server_fn_macro_default::server;
// /// # #[cfg(feature = "browser")] {
// /// use server_fn::{ServerFnError, BoxedStream, Websocket, codec::JsonEncoding};
// /// use serde::{Serialize, Deserialize};
// ///
// /// #[derive(Clone, Serialize, Deserialize)]
// /// pub struct Message {
// ///     user: String,
// ///     message: String,
// /// }
// /// // The websocket protocol can be used on any server function that accepts and returns a [`BoxedStream`]
// /// // with items that can be encoded by the input and output encoding generics.
// /// //
// /// // In this case, the input and output encodings are [`Json`] and [`Json`], respectively which requires
// /// // the items to implement [`Serialize`] and [`Deserialize`].
// /// #[server(protocol = Websocket<JsonEncoding, JsonEncoding>)]
// /// async fn echo_websocket(
// ///     input: BoxedStream<Message, ServerFnError>,
// /// ) -> Result<BoxedStream<Message, ServerFnError>, ServerFnError> {
// ///     Ok(input.into())
// /// }
// /// # }
// /// ```
// pub struct Websocket<InputEncoding, OutputEncoding>(PhantomData<(InputEncoding, OutputEncoding)>);

// /// A boxed stream type that can be used with the websocket protocol.
// ///
// /// You can easily convert any static type that implement [`futures::Stream`] into a [`BoxedStream`]
// /// with the [`From`] trait.
// ///
// /// # Example
// ///
// /// ```rust, no_run
// /// use futures::StreamExt;
// /// use server_fn::{BoxedStream, ServerFnError};
// ///
// /// let stream: BoxedStream<_, ServerFnError> =
// ///     futures::stream::iter(0..10).map(Result::Ok).into();
// /// ```
// pub struct BoxedStream<T, E = HybridError> {
//     stream: Pin<Box<dyn Stream<Item = Result<T, E>> + Send>>,
// }

// impl<T, E> From<BoxedStream<T, E>> for Pin<Box<dyn Stream<Item = Result<T, E>> + Send>> {
//     fn from(val: BoxedStream<T, E>) -> Self {
//         val.stream
//     }
// }

// impl<T, E> Deref for BoxedStream<T, E> {
//     type Target = Pin<Box<dyn Stream<Item = Result<T, E>> + Send>>;
//     fn deref(&self) -> &Self::Target {
//         &self.stream
//     }
// }

// impl<T, E> DerefMut for BoxedStream<T, E> {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.stream
//     }
// }

// impl<T, E> Debug for BoxedStream<T, E> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("BoxedStream").finish()
//     }
// }

// impl<T, E, S> From<S> for BoxedStream<T, E>
// where
//     S: Stream<Item = Result<T, E>> + Send + 'static,
// {
//     fn from(stream: S) -> Self {
//         BoxedStream {
//             stream: Box::pin(stream),
//         }
//     }
// }

// type InputStreamError = HybridError;
// type OutputStreamError = HybridError;

// impl<
//         Input,
//         InputItem,
//         OutputItem,
//         InputEncoding,
//         OutputEncoding,
//         // Error,
//         // InputStreamError,
//         // OutputStreamError,
//     >
//     Protocol<
//         Input,
//         BoxedStream<OutputItem, OutputStreamError>,
//         // Error,
//         // InputStreamError,
//         // OutputStreamError,
//     > for Websocket<InputEncoding, OutputEncoding>
// where
//     Input: Deref<Target = BoxedStream<InputItem, InputStreamError>>
//         + Into<BoxedStream<InputItem, InputStreamError>>
//         + From<BoxedStream<InputItem, InputStreamError>>,
//     InputEncoding: Encodes<InputItem> + Decodes<InputItem>,
//     OutputEncoding: Encodes<OutputItem> + Decodes<OutputItem>,
//     // InputStreamError: FromServerFnError + Send,
//     // OutputStreamError: FromServerFnError + Send,
//     // Error: FromServerFnError + Send,
//     OutputItem: Send + 'static,
//     InputItem: Send + 'static,
// {
//     const METHOD: Method = Method::GET;

//     async fn run_server<F, Fut>(
//         request: HybridRequest,
//         server_fn: F,
//     ) -> Result<HybridResponse, HybridError>
//     where
//         F: Fn(Input) -> Fut + Send,
//         Fut: Future<Output = Result<BoxedStream<OutputItem, OutputStreamError>, HybridError>>,
//     {
//         let (request_bytes, response_stream, response) = request.try_into_websocket().await?;
//         let input = request_bytes.map(|request_bytes| {
//             let request_bytes = request_bytes
//                 .map(|bytes| crate::deserialize_result::<InputStreamError>(bytes))
//                 .unwrap_or_else(Err);
//             match request_bytes {
//                 Ok(request_bytes) => InputEncoding::decode(request_bytes).map_err(|e| {
//                     InputStreamError::from_server_fn_error(ServerFnError::Deserialization(
//                         e.to_string(),
//                     ))
//                 }),
//                 Err(err) => Err(InputStreamError::de(err)),
//             }
//         });
//         let boxed = Box::pin(input)
//             as Pin<Box<dyn Stream<Item = Result<InputItem, InputStreamError>> + Send>>;
//         let input = BoxedStream { stream: boxed };

//         let output = server_fn(input.into()).await?;

//         let output = output.stream.map(|output| {
//             let result = match output {
//                 Ok(output) => OutputEncoding::encode(&output).map_err(|e| {
//                     OutputStreamError::from_server_fn_error(ServerFnError::Serialization(
//                         e.to_string(),
//                     ))
//                     .ser()
//                 }),
//                 Err(err) => Err(err.ser()),
//             };
//             crate::serialize_result(result)
//         });

//         todo!("Spawn a stream");
//         // Server::spawn(async move {
//         //     pin_mut!(response_stream);
//         //     pin_mut!(output);
//         //     while let Some(output) = output.next().await {
//         //         if response_stream.send(output).await.is_err() {
//         //             break;
//         //         }
//         //     }
//         // })?;

//         Ok(HybridResponse { res: response })
//     }

//     fn run_client(
//         path: &str,
//         input: Input,
//     ) -> impl Future<Output = Result<BoxedStream<OutputItem, OutputStreamError>, HybridError>> + Send
//     {
//         let input = input.into();

//         async move {
//             todo!()
//             // let (stream, sink) = Client::open_websocket(path).await?;

//             // // Forward the input stream to the websocket
//             // Client::spawn(async move {
//             //     pin_mut!(input);
//             //     pin_mut!(sink);
//             //     while let Some(input) = input.stream.next().await {
//             //         let result = match input {
//             //             Ok(input) => InputEncoding::encode(&input).map_err(|e| {
//             //                 InputStreamError::from_server_fn_error(ServerFnError::Serialization(
//             //                     e.to_string(),
//             //                 ))
//             //                 .ser()
//             //             }),
//             //             Err(err) => Err(err.ser()),
//             //         };
//             //         let result = serialize_result(result);
//             //         if sink.send(result).await.is_err() {
//             //             break;
//             //         }
//             //     }
//             // });

//             // // Return the output stream
//             // let stream = stream.map(|request_bytes| {
//             //     let request_bytes = request_bytes
//             //         .map(|bytes| deserialize_result::<OutputStreamError>(bytes))
//             //         .unwrap_or_else(Err);
//             //     match request_bytes {
//             //         Ok(request_bytes) => OutputEncoding::decode(request_bytes).map_err(|e| {
//             //             OutputStreamError::from_server_fn_error(ServerFnError::Deserialization(
//             //                 e.to_string(),
//             //             ))
//             //         }),
//             //         Err(err) => Err(OutputStreamError::de(err)),
//             //     }
//             // });
//             // let boxed = Box::pin(stream)
//             //     as Pin<Box<dyn Stream<Item = Result<OutputItem, OutputStreamError>> + Send>>;
//             // let output = BoxedStream { stream: boxed };
//             // Ok(output)
//         }
//     }
// }

use base64::{engine::general_purpose::STANDARD_NO_PAD, DecodeError, Engine};

use crate::{
    codec::{FromReq, FromRes, IntoReq, IntoRes},
    ContentType, Decodes, Encodes, FormatType, FromServerFnError, HybridError, HybridRequest,
    HybridResponse, ServerFnError,
};

// use super::client::Client;
use super::codec::Encoding;
// use super::codec::{Encoding, FromReq, FromRes, IntoReq, IntoRes};

// #[cfg(feature = "form-redirects")]
// use super::error::ServerFnUrlError;

// use super::middleware::{BoxedService, Layer, Service};
use super::redirect::call_redirect_hook;
// use super::response::{Res, TryRes};
// use super::response::{ClientRes, Res, TryRes};
use bytes::{BufMut, Bytes, BytesMut};
use dashmap::DashMap;
use futures::{pin_mut, SinkExt, Stream, StreamExt};
use http::{method, Method};

// use super::server::Server;
use std::{
    fmt::{Debug, Display},
    future::Future,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    pin::Pin,
    sync::{Arc, LazyLock},
};



//! This module uses platform-agnostic abstractions
//! allowing users to run server functions on a wide range of
//! platforms.
//!
//! The crates in use in this crate are:
//!
//! * `bytes`: platform-agnostic manipulation of bytes.
//! * `http`: low-dependency HTTP abstractions' *front-end*.
//!
//! # Users
//!
//! * `wasm32-wasip*` integration crate `leptos_wasi` is using this
//!   crate under the hood.

use crate::{
    error::{FromServerFnError, IntoAppError, ServerFnError},
    request::Req,
};
use bytes::Bytes;
use futures::{
    stream::{self, Stream},
    Sink, StreamExt,
};
use http::{Request, Response};
use std::borrow::Cow;

impl<Error, InputStreamError, OutputStreamError>
    Req<Error, InputStreamError, OutputStreamError> for Request<Bytes>
where
    Error: FromServerFnError + Send,
    InputStreamError: FromServerFnError + Send,
    OutputStreamError: FromServerFnError + Send,
{
    type WebsocketResponse = Response<Bytes>;

    async fn try_into_bytes(self) -> Result<Bytes, Error> {
        Ok(self.into_body())
    }

    async fn try_into_string(self) -> Result<String, Error> {
        String::from_utf8(self.into_body().into()).map_err(|err| {
            ServerFnError::Deserialization(err.to_string()).into_app_error()
        })
    }

    fn try_into_stream(
        self,
    ) -> Result<impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static, Error>
    {
        Ok(stream::iter(self.into_body())
            .ready_chunks(16)
            .map(|chunk| Ok(Bytes::from(chunk))))
    }

    fn to_content_type(&self) -> Option<Cow<'_, str>> {
        self.headers()
            .get(http::header::CONTENT_TYPE)
            .map(|val| String::from_utf8_lossy(val.as_bytes()))
    }

    fn accepts(&self) -> Option<Cow<'_, str>> {
        self.headers()
            .get(http::header::ACCEPT)
            .map(|val| String::from_utf8_lossy(val.as_bytes()))
    }

    fn referer(&self) -> Option<Cow<'_, str>> {
        self.headers()
            .get(http::header::REFERER)
            .map(|val| String::from_utf8_lossy(val.as_bytes()))
    }

    fn as_query(&self) -> Option<&str> {
        self.uri().query()
    }

    async fn try_into_websocket(
        self,
    ) -> Result<
        (
            impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static,
            impl Sink<Bytes> + Send + 'static,
            Self::WebsocketResponse,
        ),
        Error,
    > {
        Err::<
            (
                futures::stream::Once<std::future::Ready<Result<Bytes, Bytes>>>,
                futures::sink::Drain<Bytes>,
                Self::WebsocketResponse,
            ),
            _,
        >(Error::from_server_fn_error(
            crate::ServerFnError::Response(
                "Websockets are not supported on this platform.".to_string(),
            ),
        ))
    }
}


//! This module uses platform-agnostic abstractions
//! allowing users to run server functions on a wide range of
//! platforms.
//!
//! The crates in use in this crate are:
//!
//! * `bytes`: platform-agnostic manipulation of bytes.
//! * `http`: low-dependency HTTP abstractions' *front-end*.
//!
//! # Users
//!
//! * `wasm32-wasip*` integration crate `leptos_wasi` is using this
//!   crate under the hood.

use super::{Res, TryRes};
use crate::error::{
    FromServerFnError, IntoAppError, ServerFnError, ServerFnErrorWrapper, SERVER_FN_ERROR_HEADER,
};
use bytes::Bytes;
use futures::{Stream, TryStreamExt};
use http::{header, HeaderValue, Response, StatusCode};
use std::pin::Pin;
use throw_error::Error;

/// The Body of a Response whose *execution model* can be
/// customised using the variants.
pub enum Body {
    /// The response body will be written synchronously.
    Sync(Bytes),

    /// The response body will be written asynchronously,
    /// this execution model is also known as
    /// "streaming".
    Async(Pin<Box<dyn Stream<Item = Result<Bytes, Error>> + Send + 'static>>),
}

impl From<String> for Body {
    fn from(value: String) -> Self {
        Body::Sync(Bytes::from(value))
    }
}

impl From<Bytes> for Body {
    fn from(value: Bytes) -> Self {
        Body::Sync(value)
    }
}

impl<E> TryRes<E> for Response<Body>
where
    E: Send + Sync + FromServerFnError,
{
    fn try_from_string(content_type: &str, data: String) -> Result<Self, E> {
        let builder = http::Response::builder();
        builder
            .status(200)
            .header(http::header::CONTENT_TYPE, content_type)
            .body(data.into())
            .map_err(|e| ServerFnError::Response(e.to_string()).into_app_error())
    }

    fn try_from_bytes(content_type: &str, data: Bytes) -> Result<Self, E> {
        let builder = http::Response::builder();
        builder
            .status(200)
            .header(http::header::CONTENT_TYPE, content_type)
            .body(Body::Sync(data))
            .map_err(|e| ServerFnError::Response(e.to_string()).into_app_error())
    }

    fn try_from_stream(
        content_type: &str,
        data: impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static,
    ) -> Result<Self, E> {
        let builder = http::Response::builder();
        builder
            .status(200)
            .header(http::header::CONTENT_TYPE, content_type)
            .body(Body::Async(Box::pin(
                data.map_err(|e| ServerFnErrorWrapper(E::de(e)))
                    .map_err(Error::from),
            )))
            .map_err(|e| ServerFnError::Response(e.to_string()).into_app_error())
    }
}

impl Res for Response<Body> {
    fn error_response(path: &str, err: Bytes) -> Self {
        Response::builder()
            .status(http::StatusCode::INTERNAL_SERVER_ERROR)
            .header(SERVER_FN_ERROR_HEADER, path)
            .body(err.into())
            .unwrap()
    }

    fn redirect(&mut self, path: &str) {
        if let Ok(path) = HeaderValue::from_str(path) {
            self.headers_mut().insert(header::LOCATION, path);
            *self.status_mut() = StatusCode::FOUND;
        }
    }
}


/// A trait for types that can be returned from a server function.
pub trait FromServerFnError: Debug + Sized + 'static {
    /// The encoding strategy used to serialize and deserialize this error type. Must implement the [`Encodes`](server_fn::Encodes) trait for references to the error type.
    type Encoder: Encodes<Self> + Decodes<Self>;

    /// Converts a [`ServerFnError`] into the application-specific custom error type.
    fn from_server_fn_error(value: ServerFnError) -> Self;

    /// Converts the custom error type to a [`String`].
    fn ser(&self) -> Bytes {
        Self::Encoder::encode(self).unwrap_or_else(|e| {
            Self::Encoder::encode(&Self::from_server_fn_error(ServerFnError::Serialization(
                e.to_string(),
            )))
            .expect(
                "error serializing should success at least with the \
                 Serialization error",
            )
        })
    }

    /// Deserializes the custom error type from a [`&str`].
    fn de(data: Bytes) -> Self {
        Self::Encoder::decode(data)
            .unwrap_or_else(|e| ServerFnError::Deserialization(e.to_string()).into_app_error())
    }
}

/// A helper trait for converting a [`ServerFnError`] into an application-specific custom error type that implements [`FromServerFnError`].
pub trait IntoAppError<E> {
    /// Converts a [`ServerFnError`] into the application-specific custom error type.
    fn into_app_error(self) -> E;
}

impl<E> IntoAppError<E> for ServerFnError
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
                return ServerFnError::Deserialization(err.to_string()).into_app_error();
            }
        };
        E::de(decoded.into())
    }
}


// impl HybridResponse {
// /// Attempts to extract a UTF-8 string from an HTTP response.
// pub async fn try_into_string(self) -> Result<String, ServerFnError> {
//     todo!()
// }

// /// Attempts to extract a binary blob from an HTTP response.
// pub async fn try_into_bytes(self) -> Result<Bytes, ServerFnError> {
//     todo!()
// }

// /// Attempts to extract a binary stream from an HTTP response.
// pub fn try_into_stream(
//     self,
// ) -> Result<impl Stream<Item = Result<Bytes, Bytes>> + Send + Sync + 'static, ServerFnError> {
//     Ok(async { todo!() }.into_stream())
// }

// /// HTTP status code of the response.
// pub fn status(&self) -> u16 {
//     todo!()
// }

// /// Status text for the status code.
// pub fn status_text(&self) -> String {
//     todo!()
// }

// /// The `Location` header or (if none is set), the URL of the response.
// pub fn location(&self) -> String {
//     todo!()
// }

// /// Whether the response has the [`REDIRECT_HEADER`](crate::redirect::REDIRECT_HEADER) set.
// pub fn has_redirect(&self) -> bool {
//     todo!()
// }
// }



fn it_works() {
    // let a = verify(handler_implicit);
    let a = verify(handler_explicit);
    let b = verify(handler_implicit_result);

    // <handler_explicit as IntoServerFnResponse<AxumMarker>>;
}

fn verify<M, F: IntoServerFnResponse<M>>(f: impl Fn() -> F) -> M {
    todo!()
}

#[derive(serde::Serialize, serde::Deserialize)]
struct MyObject {
    id: i32,
    name: String,
}

fn handler_implicit() -> MyObject {
    todo!()
}

fn handler_implicit_result() -> Result<MyObject, ServerFnError> {
    todo!()
}

fn handler_explicit() -> Json<MyObject> {
    todo!()
}

// pub struct DefaultJsonEncoder<T>(std::marker::PhantomData<T>);

// /// Represents the response as created by the server;
// pub trait Res {
//     /// Converts an error into a response, with a `500` status code and the error text as its body.
//     fn error_response(path: &str, err: Bytes) -> Self;

//     /// Redirect the response by setting a 302 code and Location header.
//     fn redirect(&mut self, path: &str);
// }

// /// Represents the response as received by the client.
// pub trait ClientRes<E> {
//     /// Attempts to extract a UTF-8 string from an HTTP response.
//     fn try_into_string(self) -> impl Future<Output = Result<String, E>> + Send;

//     /// Attempts to extract a binary blob from an HTTP response.
//     fn try_into_bytes(self) -> impl Future<Output = Result<Bytes, E>> + Send;

//     /// Attempts to extract a binary stream from an HTTP response.
//     fn try_into_stream(
//         self,
//     ) -> Result<impl Stream<Item = Result<Bytes, Bytes>> + Send + Sync + 'static, E>;

//     /// HTTP status code of the response.
//     fn status(&self) -> u16;

//     /// Status text for the status code.
//     fn status_text(&self) -> String;

//     /// The `Location` header or (if none is set), the URL of the response.
//     fn location(&self) -> String;

//     /// Whether the response has the [`REDIRECT_HEADER`](crate::redirect::REDIRECT_HEADER) set.
//     fn has_redirect(&self) -> bool;
// }

// /// A mocked response type that can be used in place of the actual server response,
// /// when compiling for the browser.
// ///
// /// ## Panics
// /// This always panics if its methods are called. It is used solely to stub out the
// /// server response type when compiling for the client.
// pub struct BrowserMockRes;

// impl<E> TryRes<E> for BrowserMockRes {
//     fn try_from_string(_content_type: &str, _data: String) -> Result<Self, E> {
//         unreachable!()
//     }

//     fn try_from_bytes(_content_type: &str, _data: Bytes) -> Result<Self, E> {
//         unreachable!()
//     }

//     fn try_from_stream(
//         _content_type: &str,
//         _data: impl Stream<Item = Result<Bytes, Bytes>>,
//     ) -> Result<Self, E> {
//         unreachable!()
//     }
// }

// impl Res for BrowserMockRes {
//     fn error_response(_path: &str, _err: Bytes) -> Self {
//         unreachable!()
//     }

//     fn redirect(&mut self, _path: &str) {
//         unreachable!()
//     }
// }

// /// Represents the response as created by the server;
// pub trait TryRes<E>
// where
//     Self: Sized,
// {
//     /// Attempts to convert a UTF-8 string into an HTTP response.
//     fn try_from_string(content_type: &str, data: String) -> Result<Self, E>;

//     /// Attempts to convert a binary blob represented as bytes into an HTTP response.
//     fn try_from_bytes(content_type: &str, data: Bytes) -> Result<Self, E>;

//     /// Attempts to convert a stream of bytes into an HTTP response.
//     fn try_from_stream(
//         content_type: &str,
//         data: impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static,
//     ) -> Result<Self, E>;
// }


use crate::ServerFnError;

pub trait IntoServerFnResponse<Marker> {}

pub struct AxumMarker;
impl<T> IntoServerFnResponse<AxumMarker> for T where T: axum::response::IntoResponse {}

pub struct MyWebSocket {}
pub struct MyWebSocketMarker;
impl IntoServerFnResponse<MyWebSocketMarker> for MyWebSocket {}

pub struct DefaultEncodingMarker;
impl<T: 'static> IntoServerFnResponse<DefaultEncodingMarker> for Result<T, ServerFnError> where
    T: serde::Serialize
{
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


// use super::{Res, TryRes};
use crate::error::{FromServerFnError, IntoAppError, ServerFnError, SERVER_FN_ERROR_HEADER};
// ServerFnErrorWrapper,
use axum::body::Body;
use bytes::Bytes;
use futures::{Stream, TryStreamExt};
use http::{header, HeaderValue, Response, StatusCode};

// impl<E> TryRes<E> for Response<Body>
// where
//     E: Send + Sync + FromServerFnError,
// {
//     fn try_from_string(content_type: &str, data: String) -> Result<Self, E> {
//         let builder = http::Response::builder();
//         builder
//             .status(200)
//             .header(http::header::CONTENT_TYPE, content_type)
//             .body(Body::from(data))
//             .map_err(|e| ServerFnError::Response(e.to_string()).into_app_error())
//     }

//     fn try_from_bytes(content_type: &str, data: Bytes) -> Result<Self, E> {
//         let builder = http::Response::builder();
//         builder
//             .status(200)
//             .header(http::header::CONTENT_TYPE, content_type)
//             .body(Body::from(data))
//             .map_err(|e| ServerFnError::Response(e.to_string()).into_app_error())
//     }

//     fn try_from_stream(
//         content_type: &str,
//         data: impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static,
//     ) -> Result<Self, E> {
//         let body = Body::from_stream(data.map_err(|e| ServerFnErrorWrapper(E::de(e))));
//         let builder = http::Response::builder();
//         builder
//             .status(200)
//             .header(http::header::CONTENT_TYPE, content_type)
//             .body(body)
//             .map_err(|e| ServerFnError::Response(e.to_string()).into_app_error())
//     }
// }

// impl Res for Response<Body> {
//     fn error_response(path: &str, err: Bytes) -> Self {
//         Response::builder()
//             .status(http::StatusCode::INTERNAL_SERVER_ERROR)
//             .header(SERVER_FN_ERROR_HEADER, path)
//             .body(err.into())
//             .unwrap()
//     }

//     fn redirect(&mut self, path: &str) {
//         if let Ok(path) = HeaderValue::from_str(path) {
//             self.headers_mut().insert(header::LOCATION, path);
//             *self.status_mut() = StatusCode::FOUND;
//         }
//     }
// }


use super::ClientRes;
use crate::{
    error::{FromServerFnError, IntoAppError, ServerFnError},
    redirect::REDIRECT_HEADER,
};
use bytes::Bytes;
use futures::{Stream, StreamExt};
pub use gloo_net::http::Response;
use http::{HeaderMap, HeaderName, HeaderValue};
use js_sys::Uint8Array;
use send_wrapper::SendWrapper;
use std::{future::Future, str::FromStr};
use wasm_bindgen::JsCast;
use wasm_streams::ReadableStream;

/// The response to a `fetch` request made in the browser.
pub struct BrowserResponse(pub(crate) SendWrapper<Response>);

impl BrowserResponse {
    /// Generate the headers from the internal [`Response`] object.
    /// This is a workaround for the fact that the `Response` object does not
    /// have a [`HeaderMap`] directly. This function will iterate over the
    /// headers and convert them to a [`HeaderMap`].
    pub fn generate_headers(&self) -> HeaderMap {
        self.0
            .headers()
            .entries()
            .filter_map(|(key, value)| {
                let key = HeaderName::from_str(&key).ok()?;
                let value = HeaderValue::from_str(&value).ok()?;
                Some((key, value))
            })
            .collect()
    }
}

impl<E: FromServerFnError> ClientRes<E> for BrowserResponse {
    fn try_into_string(self) -> impl Future<Output = Result<String, E>> + Send {
        // the browser won't send this async work between threads (because it's single-threaded)
        // so we can safely wrap this
        SendWrapper::new(async move {
            self.0
                .text()
                .await
                .map_err(|e| ServerFnError::Deserialization(e.to_string()).into_app_error())
        })
    }

    fn try_into_bytes(self) -> impl Future<Output = Result<Bytes, E>> + Send {
        // the browser won't send this async work between threads (because it's single-threaded)
        // so we can safely wrap this
        SendWrapper::new(async move {
            self.0
                .binary()
                .await
                .map(Bytes::from)
                .map_err(|e| ServerFnError::Deserialization(e.to_string()).into_app_error())
        })
    }

    fn try_into_stream(
        self,
    ) -> Result<impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static, E> {
        let stream = ReadableStream::from_raw(self.0.body().unwrap())
            .into_stream()
            .map(|data| match data {
                Err(e) => {
                    web_sys::console::error_1(&e);
                    Err(E::from_server_fn_error(ServerFnError::Request(format!("{e:?}"))).ser())
                }
                Ok(data) => {
                    let data = data.unchecked_into::<Uint8Array>();
                    let mut buf = Vec::new();
                    let length = data.length();
                    buf.resize(length as usize, 0);
                    data.copy_to(&mut buf);
                    Ok(Bytes::from(buf))
                }
            });
        Ok(SendWrapper::new(stream))
    }

    fn status(&self) -> u16 {
        self.0.status()
    }

    fn status_text(&self) -> String {
        self.0.status_text()
    }

    fn location(&self) -> String {
        self.0
            .headers()
            .get("Location")
            .unwrap_or_else(|| self.0.url())
    }

    fn has_redirect(&self) -> bool {
        self.0.headers().get(REDIRECT_HEADER).is_some()
    }
}
