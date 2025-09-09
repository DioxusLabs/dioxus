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

// /// The protocol that a server function uses to communicate with the client. This trait handles
// /// the server and client side of running a server function. It is implemented for the [`Http`] and
// /// [`Websocket`] protocols and can be used to implement custom protocols.
// pub trait Protocol<Input, Output> {
//     /// The HTTP method used for requests.
//     const METHOD: Method;

//     /// Run the server function on the server. The implementation should handle deserializing the
//     /// input, running the server function, and serializing the output.
//     async fn run_server<F, Fut>(
//         request: HybridRequest,
//         server_fn: F,
//     ) -> Result<HybridResponse, HybridError>
//     where
//         F: Fn(Input) -> Fut + Send,
//         Fut: Future<Output = Result<Output, HybridError>>;

//     /// Run the server function on the client. The implementation should handle serializing the
//     /// input, sending the request, and deserializing the output.
//     async fn run_client(path: &str, input: Input) -> Result<Output, HybridError>;
// }
