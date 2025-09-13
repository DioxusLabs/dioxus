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
