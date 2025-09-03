#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! # Server Functions
//!
//! This package is based on a simple idea: sometimes it’s useful to write functions
//! that will only run on the server, and call them from the client.
//!
//! If you’re creating anything beyond a toy app, you’ll need to do this all the time:
//! reading from or writing to a database that only runs on the server, running expensive
//! computations using libraries you don’t want to ship down to the client, accessing
//! APIs that need to be called from the server rather than the client for CORS reasons
//! or because you need a secret API key that’s stored on the server and definitely
//! shouldn’t be shipped down to a user’s browser.
//!
//! Traditionally, this is done by separating your server and client code, and by setting
//! up something like a REST API or GraphQL API to allow your client to fetch and mutate
//! data on the server. This is fine, but it requires you to write and maintain your code
//! in multiple separate places (client-side code for fetching, server-side functions to run),
//! as well as creating a third thing to manage, which is the API contract between the two.
//!
//! This package provides two simple primitives that allow you instead to write co-located,
//! isomorphic server functions. (*Co-located* means you can write them in your app code so
//! that they are “located alongside” the client code that calls them, rather than separating
//! the client and server sides. *Isomorphic* means you can call them from the client as if
//! you were simply calling a function; the function call has the “same shape” on the client
//! as it does on the server.)
//!
//! ### `#[server]`
//!
//! The [`#[server]`](../leptos/attr.server.html) macro allows you to annotate a function to
//! indicate that it should only run on the server (i.e., when you have an `ssr` feature in your
//! crate that is enabled).
//!
//! **Important**: Before calling a server function on a non-web platform, you must set the server URL by calling
//! [`set_server_url`](crate::client::set_server_url).
//!
//! ```rust,ignore
//! #[server]
//! async fn read_posts(how_many: usize, query: String) -> Result<Vec<Posts>, ServerFnError> {
//!   // do some server-only work here to access the database
//!   let posts = ...;
//!   Ok(posts)
//! }
//!
//! // call the function
//! # #[tokio::main]
//! # async fn main() {
//! async {
//!   let posts = read_posts(3, "my search".to_string()).await;
//!   log::debug!("posts = {posts:#?}");
//! }
//! # }
//! ```
//!
//! If you call this function from the client, it will serialize the function arguments and `POST`
//! them to the server as if they were the URL-encoded inputs in `<form method="post">`.
//!
//! Here’s what you need to remember:
//! - **Server functions must be `async`.** Even if the work being done inside the function body
//!   can run synchronously on the server, from the client’s perspective it involves an asynchronous
//!   function call.
//! - **Server functions must return `Result<T, ServerFnError>`.** Even if the work being done
//!   inside the function body can’t fail, the processes of serialization/deserialization and the
//!   network call are fallible. [`ServerFnError`] can receive generic errors.
//! - **Server functions are part of the public API of your application.** A server function is an
//!   ad hoc HTTP API endpoint, not a magic formula. Any server function can be accessed by any HTTP
//!   client. You should take care to sanitize any data being returned from the function to ensure it
//!   does not leak data that should exist only on the server.
//! - **Server functions can’t be generic.** Because each server function creates a separate API endpoint,
//!   it is difficult to monomorphize. As a result, server functions cannot be generic (for now?) If you need to use
//!   a generic function, you can define a generic inner function called by multiple concrete server functions.
//! - **Arguments and return types must be serializable.** We support a variety of different encodings,
//!   but one way or another arguments need to be serialized to be sent to the server and deserialized
//!   on the server, and the return type must be serialized on the server and deserialized on the client.
//!   This means that the set of valid server function argument and return types is a subset of all
//!   possible Rust argument and return types. (i.e., server functions are strictly more limited than
//!   ordinary functions.)
//!
//! ## Server Function Encodings
//!
//! Server functions are designed to allow a flexible combination of input and output encodings, the set
//! of which can be found in the [`codec`] module.
//!
//! Calling and handling server functions is done through the [`Protocol`] trait, which is implemented
//! for the [`Http`] and [`Websocket`] protocols. Most server functions will use the [`Http`] protocol.
//!
//! When using the [`Http`] protocol, the serialization/deserialization process for server functions
//! consists of a series of steps, each of which is represented by a different trait:
//! 1. [`IntoReq`]: The client serializes the [`ServerFn`] argument type into an HTTP request.
//! 2. The [`Client`] sends the request to the server.
//! 3. [`FromReq`]: The server deserializes the HTTP request back into the [`ServerFn`] type.
//! 4. The server calls calls [`ServerFn::run_body`] on the data.
//! 5. [`IntoRes`]: The server serializes the [`ServerFn::Output`] type into an HTTP response.
//! 6. The server integration applies any middleware from [`ServerFn::middlewares`] and responds to the request.
//! 7. [`FromRes`]: The client deserializes the response back into the [`ServerFn::Output`] type.
//!
//! [server]: ../leptos/attr.server.html
//! [`serde_qs`]: <https://docs.rs/serde_qs/latest/serde_qs/>
//! [`cbor`]: <https://docs.rs/cbor/latest/cbor/>

/// Implementations of the client side of the server function call.
pub mod client;

/// Implementations of the server side of the server function call.
pub mod server;

/// Encodings for arguments and results.
pub mod codec;

#[macro_use]
/// Error types and utilities.
pub mod error;
/// Types to add server middleware to a server function.
pub mod middleware;
/// Utilities to allow client-side redirects.
pub mod redirect;
/// Types and traits for  for HTTP requests.
pub mod request;
/// Types and traits for HTTP responses.
pub mod response;

#[cfg(feature = "actix-no-default")]
#[doc(hidden)]
pub use ::actix_web as actix_export;
#[cfg(feature = "axum-no-default")]
#[doc(hidden)]
pub use ::axum as axum_export;
#[cfg(feature = "generic")]
#[doc(hidden)]
pub use ::bytes as bytes_export;
#[cfg(feature = "generic")]
#[doc(hidden)]
pub use ::http as http_export;
use base64::{engine::general_purpose::STANDARD_NO_PAD, DecodeError, Engine};
// re-exported to make it possible to implement a custom Client without adding a separate
// dependency on `bytes`
pub use bytes::Bytes;
use bytes::{BufMut, BytesMut};
use client::Client;
use codec::{Encoding, FromReq, FromRes, IntoReq, IntoRes};
#[doc(hidden)]
pub use const_format;
#[doc(hidden)]
pub use const_str;
use dashmap::DashMap;
pub use error::ServerFnError;
#[cfg(feature = "form-redirects")]
use error::ServerFnUrlError;
use error::{FromServerFnError, ServerFnErrorErr};
use futures::{pin_mut, SinkExt, Stream, StreamExt};
use http::Method;
use middleware::{BoxedService, Layer, Service};
use redirect::call_redirect_hook;
use request::Req;
use response::{ClientRes, Res, TryRes};
#[cfg(feature = "rkyv")]
pub use rkyv;
#[doc(hidden)]
pub use serde;
#[doc(hidden)]
#[cfg(feature = "serde-lite")]
pub use serde_lite;
use server::Server;
use std::{
    fmt::{Debug, Display},
    future::Future,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    pin::Pin,
    sync::{Arc, LazyLock},
};
#[doc(hidden)]
pub use xxhash_rust;

type ServerFnServerRequest<Fn> = <<Fn as ServerFn>::Server as crate::Server<
    <Fn as ServerFn>::Error,
    <Fn as ServerFn>::InputStreamError,
    <Fn as ServerFn>::OutputStreamError,
>>::Request;
type ServerFnServerResponse<Fn> = <<Fn as ServerFn>::Server as crate::Server<
    <Fn as ServerFn>::Error,
    <Fn as ServerFn>::InputStreamError,
    <Fn as ServerFn>::OutputStreamError,
>>::Response;

/// Defines a function that runs only on the server, but can be called from the server or the client.
///
/// The type for which `ServerFn` is implemented is actually the type of the arguments to the function,
/// while the function body itself is implemented in [`run_body`](ServerFn::run_body).
///
/// This means that `Self` here is usually a struct, in which each field is an argument to the function.
/// In other words,
/// ```rust,ignore
/// #[server]
/// pub async fn my_function(foo: String, bar: usize) -> Result<usize, ServerFnError> {
///     Ok(foo.len() + bar)
/// }
/// ```
/// should expand to
/// ```rust,ignore
/// #[derive(Serialize, Deserialize)]
/// pub struct MyFunction {
///     foo: String,
///     bar: usize
/// }
///
/// impl ServerFn for MyFunction {
///     async fn run_body() -> Result<usize, ServerFnError> {
///         Ok(foo.len() + bar)
///     }
///
///     // etc.
/// }
/// ```
pub trait ServerFn: Send + Sized {
    /// A unique path for the server function’s API endpoint, relative to the host, including its prefix.
    const PATH: &'static str;

    /// The type of the HTTP client that will send the request from the client side.
    ///
    /// For example, this might be `gloo-net` in the browser, or `reqwest` for a desktop app.
    type Client: Client<
        Self::Error,
        Self::InputStreamError,
        Self::OutputStreamError,
    >;

    /// The type of the HTTP server that will send the response from the server side.
    ///
    /// For example, this might be `axum` or `actix-web`.
    type Server: Server<
        Self::Error,
        Self::InputStreamError,
        Self::OutputStreamError,
    >;

    /// The protocol the server function uses to communicate with the client.
    type Protocol: Protocol<
        Self,
        Self::Output,
        Self::Client,
        Self::Server,
        Self::Error,
        Self::InputStreamError,
        Self::OutputStreamError,
    >;

    /// The return type of the server function.
    ///
    /// This needs to be converted into `ServerResponse` on the server side, and converted
    /// *from* `ClientResponse` when received by the client.
    type Output: Send;

    /// The type of error in the server function return.
    /// Typically [`ServerFnError`], but allowed to be any type that implements [`FromServerFnError`].
    type Error: FromServerFnError + Send + Sync;
    /// The type of error in the server function for stream items sent from the client to the server.
    /// Typically [`ServerFnError`], but allowed to be any type that implements [`FromServerFnError`].
    type InputStreamError: FromServerFnError + Send + Sync;
    /// The type of error in the server function for stream items sent from the server to the client.
    /// Typically [`ServerFnError`], but allowed to be any type that implements [`FromServerFnError`].
    type OutputStreamError: FromServerFnError + Send + Sync;

    /// Returns [`Self::PATH`].
    fn url() -> &'static str {
        Self::PATH
    }

    /// Middleware that should be applied to this server function.
    fn middlewares() -> Vec<
        Arc<
            dyn Layer<
                ServerFnServerRequest<Self>,
                ServerFnServerResponse<Self>,
            >,
        >,
    > {
        Vec::new()
    }

    /// The body of the server function. This will only run on the server.
    fn run_body(
        self,
    ) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send;

    #[doc(hidden)]
    fn run_on_server(
        req: ServerFnServerRequest<Self>,
    ) -> impl Future<Output = ServerFnServerResponse<Self>> + Send {
        // Server functions can either be called by a real Client,
        // or directly by an HTML <form>. If they're accessed by a <form>, default to
        // redirecting back to the Referer.
        #[cfg(feature = "form-redirects")]
        let accepts_html = req
            .accepts()
            .map(|n| n.contains("text/html"))
            .unwrap_or(false);
        #[cfg(feature = "form-redirects")]
        let mut referer = req.referer().as_deref().map(ToOwned::to_owned);

        async move {
            #[allow(unused_variables, unused_mut)]
            // used in form redirects feature
            let (mut res, err) =
                Self::Protocol::run_server(req, Self::run_body)
                    .await
                    .map(|res| (res, None))
                    .unwrap_or_else(|e| {
                        (
                            <<Self as ServerFn>::Server as crate::Server<
                                Self::Error,
                                Self::InputStreamError,
                                Self::OutputStreamError,
                            >>::Response::error_response(
                                Self::PATH, e.ser()
                            ),
                            Some(e),
                        )
                    });

            // if it accepts HTML, we'll redirect to the Referer
            #[cfg(feature = "form-redirects")]
            if accepts_html {
                // if it had an error, encode that error in the URL
                if let Some(err) = err {
                    if let Ok(url) = ServerFnUrlError::new(Self::PATH, err)
                        .to_url(referer.as_deref().unwrap_or("/"))
                    {
                        referer = Some(url.to_string());
                    }
                }
                // otherwise, strip error info from referer URL, as that means it's from a previous
                // call
                else if let Some(referer) = referer.as_mut() {
                    ServerFnUrlError::<Self::Error>::strip_error_info(referer)
                }

                // set the status code and Location header
                res.redirect(referer.as_deref().unwrap_or("/"));
            }

            res
        }
    }

    #[doc(hidden)]
    fn run_on_client(
        self,
    ) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send {
        async move { Self::Protocol::run_client(Self::PATH, self).await }
    }
}

/// The protocol that a server function uses to communicate with the client. This trait handles
/// the server and client side of running a server function. It is implemented for the [`Http`] and
/// [`Websocket`] protocols and can be used to implement custom protocols.
pub trait Protocol<
    Input,
    Output,
    Client,
    Server,
    Error,
    InputStreamError = Error,
    OutputStreamError = Error,
> where
    Server: crate::Server<Error, InputStreamError, OutputStreamError>,
    Client: crate::Client<Error, InputStreamError, OutputStreamError>,
{
    /// The HTTP method used for requests.
    const METHOD: Method;

    /// Run the server function on the server. The implementation should handle deserializing the
    /// input, running the server function, and serializing the output.
    fn run_server<F, Fut>(
        request: Server::Request,
        server_fn: F,
    ) -> impl Future<Output = Result<Server::Response, Error>> + Send
    where
        F: Fn(Input) -> Fut + Send,
        Fut: Future<Output = Result<Output, Error>> + Send;

    /// Run the server function on the client. The implementation should handle serializing the
    /// input, sending the request, and deserializing the output.
    fn run_client(
        path: &str,
        input: Input,
    ) -> impl Future<Output = Result<Output, Error>> + Send;
}

/// The http protocol with specific input and output encodings for the request and response. This is
/// the default protocol server functions use if no override is set in the server function macro
///
/// The http protocol accepts two generic argument that define how the input and output for a server
/// function are turned into HTTP requests and responses. For example, [`Http<GetUrl, Json>`] will
/// accept a Url encoded Get request and return a JSON post response.
///
/// # Example
///
/// ```rust, no_run
/// # use server_fn_macro_default::server;
/// use serde::{Serialize, Deserialize};
/// use server_fn::{Http, ServerFnError, codec::{Json, GetUrl}};
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// pub struct Message {
///     user: String,
///     message: String,
/// }
///
/// // The http protocol can be used on any server function that accepts and returns arguments that implement
/// // the [`IntoReq`] and [`FromRes`] traits.
/// //
/// // In this case, the input and output encodings are [`GetUrl`] and [`Json`], respectively which requires
/// // the items to implement [`IntoReq<GetUrl, ...>`] and [`FromRes<Json, ...>`]. Both of those implementations
/// // require the items to implement [`Serialize`] and [`Deserialize`].
/// # #[cfg(feature = "browser")] {
/// #[server(protocol = Http<GetUrl, Json>)]
/// async fn echo_http(
///     input: Message,
/// ) -> Result<Message, ServerFnError> {
///     Ok(input)
/// }
/// # }
/// ```
pub struct Http<InputProtocol, OutputProtocol>(
    PhantomData<(InputProtocol, OutputProtocol)>,
);

impl<InputProtocol, OutputProtocol, Input, Output, Client, Server, E>
    Protocol<Input, Output, Client, Server, E>
    for Http<InputProtocol, OutputProtocol>
where
    Input: IntoReq<InputProtocol, Client::Request, E>
        + FromReq<InputProtocol, Server::Request, E>
        + Send,
    Output: IntoRes<OutputProtocol, Server::Response, E>
        + FromRes<OutputProtocol, Client::Response, E>
        + Send,
    E: FromServerFnError,
    InputProtocol: Encoding,
    OutputProtocol: Encoding,
    Client: crate::Client<E>,
    Server: crate::Server<E>,
{
    const METHOD: Method = InputProtocol::METHOD;

    async fn run_server<F, Fut>(
        request: Server::Request,
        server_fn: F,
    ) -> Result<Server::Response, E>
    where
        F: Fn(Input) -> Fut + Send,
        Fut: Future<Output = Result<Output, E>> + Send,
    {
        let input = Input::from_req(request).await?;

        let output = server_fn(input).await?;

        let response = Output::into_res(output).await?;

        Ok(response)
    }

    async fn run_client(path: &str, input: Input) -> Result<Output, E>
    where
        Client: crate::Client<E>,
    {
        // create and send request on client
        let req = input.into_req(path, OutputProtocol::CONTENT_TYPE)?;
        let res = Client::send(req).await?;

        let status = res.status();
        let location = res.location();
        let has_redirect_header = res.has_redirect();

        // if it returns an error status, deserialize the error using the error's decoder.
        let res = if (400..=599).contains(&status) {
            Err(E::de(res.try_into_bytes().await?))
        } else {
            // otherwise, deserialize the body as is
            let output = Output::from_res(res).await?;
            Ok(output)
        }?;

        // if redirected, call the redirect hook (if that's been set)
        if (300..=399).contains(&status) || has_redirect_header {
            call_redirect_hook(&location);
        }
        Ok(res)
    }
}

/// The websocket protocol that encodes the input and output streams using a websocket connection.
///
/// The websocket protocol accepts two generic argument that define the input and output serialization
/// formats. For example, [`Websocket<CborEncoding, JsonEncoding>`] would accept a stream of Cbor-encoded messages
/// and return a stream of JSON-encoded messages.
///
/// # Example
///
/// ```rust, no_run
/// # use server_fn_macro_default::server;
/// # #[cfg(feature = "browser")] {
/// use server_fn::{ServerFnError, BoxedStream, Websocket, codec::JsonEncoding};
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Clone, Serialize, Deserialize)]
/// pub struct Message {
///     user: String,
///     message: String,
/// }
/// // The websocket protocol can be used on any server function that accepts and returns a [`BoxedStream`]
/// // with items that can be encoded by the input and output encoding generics.
/// //
/// // In this case, the input and output encodings are [`Json`] and [`Json`], respectively which requires
/// // the items to implement [`Serialize`] and [`Deserialize`].
/// #[server(protocol = Websocket<JsonEncoding, JsonEncoding>)]
/// async fn echo_websocket(
///     input: BoxedStream<Message, ServerFnError>,
/// ) -> Result<BoxedStream<Message, ServerFnError>, ServerFnError> {
///     Ok(input.into())
/// }
/// # }
/// ```
pub struct Websocket<InputEncoding, OutputEncoding>(
    PhantomData<(InputEncoding, OutputEncoding)>,
);

/// A boxed stream type that can be used with the websocket protocol.
///
/// You can easily convert any static type that implement [`futures::Stream`] into a [`BoxedStream`]
/// with the [`From`] trait.
///
/// # Example
///
/// ```rust, no_run
/// use futures::StreamExt;
/// use server_fn::{BoxedStream, ServerFnError};
///
/// let stream: BoxedStream<_, ServerFnError> =
///     futures::stream::iter(0..10).map(Result::Ok).into();
/// ```
pub struct BoxedStream<T, E> {
    stream: Pin<Box<dyn Stream<Item = Result<T, E>> + Send>>,
}

impl<T, E> From<BoxedStream<T, E>>
    for Pin<Box<dyn Stream<Item = Result<T, E>> + Send>>
{
    fn from(val: BoxedStream<T, E>) -> Self {
        val.stream
    }
}

impl<T, E> Deref for BoxedStream<T, E> {
    type Target = Pin<Box<dyn Stream<Item = Result<T, E>> + Send>>;
    fn deref(&self) -> &Self::Target {
        &self.stream
    }
}

impl<T, E> DerefMut for BoxedStream<T, E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.stream
    }
}

impl<T, E> Debug for BoxedStream<T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoxedStream").finish()
    }
}

impl<T, E, S> From<S> for BoxedStream<T, E>
where
    S: Stream<Item = Result<T, E>> + Send + 'static,
{
    fn from(stream: S) -> Self {
        BoxedStream {
            stream: Box::pin(stream),
        }
    }
}

impl<
        Input,
        InputItem,
        OutputItem,
        InputEncoding,
        OutputEncoding,
        Client,
        Server,
        Error,
        InputStreamError,
        OutputStreamError,
    >
    Protocol<
        Input,
        BoxedStream<OutputItem, OutputStreamError>,
        Client,
        Server,
        Error,
        InputStreamError,
        OutputStreamError,
    > for Websocket<InputEncoding, OutputEncoding>
where
    Input: Deref<Target = BoxedStream<InputItem, InputStreamError>>
        + Into<BoxedStream<InputItem, InputStreamError>>
        + From<BoxedStream<InputItem, InputStreamError>>,
    InputEncoding: Encodes<InputItem> + Decodes<InputItem>,
    OutputEncoding: Encodes<OutputItem> + Decodes<OutputItem>,
    InputStreamError: FromServerFnError + Send,
    OutputStreamError: FromServerFnError + Send,
    Error: FromServerFnError + Send,
    Server: crate::Server<Error, InputStreamError, OutputStreamError>,
    Client: crate::Client<Error, InputStreamError, OutputStreamError>,
    OutputItem: Send + 'static,
    InputItem: Send + 'static,
{
    const METHOD: Method = Method::GET;

    async fn run_server<F, Fut>(
        request: Server::Request,
        server_fn: F,
    ) -> Result<Server::Response, Error>
    where
        F: Fn(Input) -> Fut + Send,
        Fut: Future<
                Output = Result<
                    BoxedStream<OutputItem, OutputStreamError>,
                    Error,
                >,
            > + Send,
    {
        let (request_bytes, response_stream, response) =
            request.try_into_websocket().await?;
        let input = request_bytes.map(|request_bytes| {
            let request_bytes = request_bytes
                .map(|bytes| deserialize_result::<InputStreamError>(bytes))
                .unwrap_or_else(Err);
            match request_bytes {
                Ok(request_bytes) => InputEncoding::decode(request_bytes)
                    .map_err(|e| {
                        InputStreamError::from_server_fn_error(
                            ServerFnErrorErr::Deserialization(e.to_string()),
                        )
                    }),
                Err(err) => Err(InputStreamError::de(err)),
            }
        });
        let boxed = Box::pin(input)
            as Pin<
                Box<
                    dyn Stream<Item = Result<InputItem, InputStreamError>>
                        + Send,
                >,
            >;
        let input = BoxedStream { stream: boxed };

        let output = server_fn(input.into()).await?;

        let output = output.stream.map(|output| {
            let result = match output {
                Ok(output) => OutputEncoding::encode(&output).map_err(|e| {
                    OutputStreamError::from_server_fn_error(
                        ServerFnErrorErr::Serialization(e.to_string()),
                    )
                    .ser()
                }),
                Err(err) => Err(err.ser()),
            };
            serialize_result(result)
        });

        Server::spawn(async move {
            pin_mut!(response_stream);
            pin_mut!(output);
            while let Some(output) = output.next().await {
                if response_stream.send(output).await.is_err() {
                    break;
                }
            }
        })?;

        Ok(response)
    }

    fn run_client(
        path: &str,
        input: Input,
    ) -> impl Future<
        Output = Result<BoxedStream<OutputItem, OutputStreamError>, Error>,
    > + Send {
        let input = input.into();

        async move {
            let (stream, sink) = Client::open_websocket(path).await?;

            // Forward the input stream to the websocket
            Client::spawn(async move {
                pin_mut!(input);
                pin_mut!(sink);
                while let Some(input) = input.stream.next().await {
                    let result = match input {
                        Ok(input) => {
                            InputEncoding::encode(&input).map_err(|e| {
                                InputStreamError::from_server_fn_error(
                                    ServerFnErrorErr::Serialization(
                                        e.to_string(),
                                    ),
                                )
                                .ser()
                            })
                        }
                        Err(err) => Err(err.ser()),
                    };
                    let result = serialize_result(result);
                    if sink.send(result).await.is_err() {
                        break;
                    }
                }
            });

            // Return the output stream
            let stream = stream.map(|request_bytes| {
                let request_bytes = request_bytes
                    .map(|bytes| deserialize_result::<OutputStreamError>(bytes))
                    .unwrap_or_else(Err);
                match request_bytes {
                    Ok(request_bytes) => OutputEncoding::decode(request_bytes)
                        .map_err(|e| {
                            OutputStreamError::from_server_fn_error(
                                ServerFnErrorErr::Deserialization(
                                    e.to_string(),
                                ),
                            )
                        }),
                    Err(err) => Err(OutputStreamError::de(err)),
                }
            });
            let boxed = Box::pin(stream)
                as Pin<
                    Box<
                        dyn Stream<Item = Result<OutputItem, OutputStreamError>>
                            + Send,
                    >,
                >;
            let output = BoxedStream { stream: boxed };
            Ok(output)
        }
    }
}

// Serializes a Result<Bytes, Bytes> into a single Bytes instance.
// Format: [tag: u8][content: Bytes]
// - Tag 0: Ok variant
// - Tag 1: Err variant
fn serialize_result(result: Result<Bytes, Bytes>) -> Bytes {
    match result {
        Ok(bytes) => {
            let mut buf = BytesMut::with_capacity(1 + bytes.len());
            buf.put_u8(0); // Tag for Ok variant
            buf.extend_from_slice(&bytes);
            buf.freeze()
        }
        Err(bytes) => {
            let mut buf = BytesMut::with_capacity(1 + bytes.len());
            buf.put_u8(1); // Tag for Err variant
            buf.extend_from_slice(&bytes);
            buf.freeze()
        }
    }
}

// Deserializes a Bytes instance back into a Result<Bytes, Bytes>.
fn deserialize_result<E: FromServerFnError>(
    bytes: Bytes,
) -> Result<Bytes, Bytes> {
    if bytes.is_empty() {
        return Err(E::from_server_fn_error(
            ServerFnErrorErr::Deserialization("Data is empty".into()),
        )
        .ser());
    }

    let tag = bytes[0];
    let content = bytes.slice(1..);

    match tag {
        0 => Ok(content),
        1 => Err(content),
        _ => Err(E::from_server_fn_error(ServerFnErrorErr::Deserialization(
            "Invalid data tag".into(),
        ))
        .ser()), // Invalid tag
    }
}

/// Encode format type
pub enum Format {
    /// Binary representation
    Binary,
    /// utf-8 compatible text representation
    Text,
}
/// A trait for types with an associated content type.
pub trait ContentType {
    /// The MIME type of the data.
    const CONTENT_TYPE: &'static str;
}

/// Data format representation
pub trait FormatType {
    /// The representation type
    const FORMAT_TYPE: Format;

    /// Encodes data into a string.
    fn into_encoded_string(bytes: Bytes) -> String {
        match Self::FORMAT_TYPE {
            Format::Binary => STANDARD_NO_PAD.encode(bytes),
            Format::Text => String::from_utf8(bytes.into())
                .expect("Valid text format type with utf-8 comptabile string"),
        }
    }

    /// Decodes string to bytes
    fn from_encoded_string(data: &str) -> Result<Bytes, DecodeError> {
        match Self::FORMAT_TYPE {
            Format::Binary => {
                STANDARD_NO_PAD.decode(data).map(|data| data.into())
            }
            Format::Text => Ok(Bytes::copy_from_slice(data.as_bytes())),
        }
    }
}

/// A trait for types that can be encoded into a bytes for a request body.
pub trait Encodes<T>: ContentType + FormatType {
    /// The error type that can be returned if the encoding fails.
    type Error: Display + Debug;

    /// Encodes the given value into a bytes.
    fn encode(output: &T) -> Result<Bytes, Self::Error>;
}

/// A trait for types that can be decoded from a bytes for a response body.
pub trait Decodes<T> {
    /// The error type that can be returned if the decoding fails.
    type Error: Display;

    /// Decodes the given bytes into a value.
    fn decode(bytes: Bytes) -> Result<T, Self::Error>;
}

#[cfg(feature = "ssr")]
#[doc(hidden)]
pub use inventory;

/// Uses the `inventory` crate to initialize a map between paths and server functions.
#[macro_export]
macro_rules! initialize_server_fn_map {
    ($req:ty, $res:ty) => {
        std::sync::LazyLock::new(|| {
            $crate::inventory::iter::<ServerFnTraitObj<$req, $res>>
                .into_iter()
                .map(|obj| {
                    ((obj.path().to_string(), obj.method()), obj.clone())
                })
                .collect()
        })
    };
}

/// A list of middlewares that can be applied to a server function.
pub type MiddlewareSet<Req, Res> = Vec<Arc<dyn Layer<Req, Res>>>;

/// A trait object that allows multiple server functions that take the same
/// request type and return the same response type to be gathered into a single
/// collection.
pub struct ServerFnTraitObj<Req, Res> {
    path: &'static str,
    method: Method,
    handler: fn(Req) -> Pin<Box<dyn Future<Output = Res> + Send>>,
    middleware: fn() -> MiddlewareSet<Req, Res>,
    ser: fn(ServerFnErrorErr) -> Bytes,
}

impl<Req, Res> ServerFnTraitObj<Req, Res> {
    /// Converts the relevant parts of a server function into a trait object.
    pub const fn new<
        S: ServerFn<
            Server: crate::Server<
                S::Error,
                S::InputStreamError,
                S::OutputStreamError,
                Request = Req,
                Response = Res,
            >,
        >,
    >(
        handler: fn(Req) -> Pin<Box<dyn Future<Output = Res> + Send>>,
    ) -> Self
    where
        Req: crate::Req<
                S::Error,
                S::InputStreamError,
                S::OutputStreamError,
                WebsocketResponse = Res,
            > + Send
            + 'static,
        Res: crate::TryRes<S::Error> + Send + 'static,
    {
        Self {
            path: S::PATH,
            method: S::Protocol::METHOD,
            handler,
            middleware: S::middlewares,
            ser: |e| S::Error::from_server_fn_error(e).ser(),
        }
    }

    /// The path of the server function.
    pub fn path(&self) -> &'static str {
        self.path
    }

    /// The HTTP method the server function expects.
    pub fn method(&self) -> Method {
        self.method.clone()
    }

    /// The handler for this server function.
    pub fn handler(&self, req: Req) -> impl Future<Output = Res> + Send {
        (self.handler)(req)
    }

    /// The set of middleware that should be applied to this function.
    pub fn middleware(&self) -> MiddlewareSet<Req, Res> {
        (self.middleware)()
    }

    /// Converts the server function into a boxed service.
    pub fn boxed(self) -> BoxedService<Req, Res>
    where
        Self: Service<Req, Res>,
        Req: 'static,
        Res: 'static,
    {
        BoxedService::new(self.ser, self)
    }
}

impl<Req, Res> Service<Req, Res> for ServerFnTraitObj<Req, Res>
where
    Req: Send + 'static,
    Res: 'static,
{
    fn run(
        &mut self,
        req: Req,
        _ser: fn(ServerFnErrorErr) -> Bytes,
    ) -> Pin<Box<dyn Future<Output = Res> + Send>> {
        let handler = self.handler;
        Box::pin(async move { handler(req).await })
    }
}

impl<Req, Res> Clone for ServerFnTraitObj<Req, Res> {
    fn clone(&self) -> Self {
        Self {
            path: self.path,
            method: self.method.clone(),
            handler: self.handler,
            middleware: self.middleware,
            ser: self.ser,
        }
    }
}

#[allow(unused)] // used by server integrations
type LazyServerFnMap<Req, Res> =
    LazyLock<DashMap<(String, Method), ServerFnTraitObj<Req, Res>>>;

#[cfg(feature = "ssr")]
impl<Req: 'static, Res: 'static> inventory::Collect
    for ServerFnTraitObj<Req, Res>
{
    #[inline]
    fn registry() -> &'static inventory::Registry {
        static REGISTRY: inventory::Registry = inventory::Registry::new();
        &REGISTRY
    }
}

/// Axum integration.
#[cfg(feature = "axum-no-default")]
pub mod axum {
    use crate::{
        error::FromServerFnError, middleware::BoxedService, LazyServerFnMap,
        Protocol, Server, ServerFn, ServerFnTraitObj,
    };
    use axum::body::Body;
    use http::{Method, Request, Response, StatusCode};
    use std::future::Future;

    static REGISTERED_SERVER_FUNCTIONS: LazyServerFnMap<
        Request<Body>,
        Response<Body>,
    > = initialize_server_fn_map!(Request<Body>, Response<Body>);

    /// The axum server function backend
    pub struct AxumServerFnBackend;

    impl<Error, InputStreamError, OutputStreamError>
        Server<Error, InputStreamError, OutputStreamError>
        for AxumServerFnBackend
    where
        Error: FromServerFnError + Send + Sync,
        InputStreamError: FromServerFnError + Send + Sync,
        OutputStreamError: FromServerFnError + Send + Sync,
    {
        type Request = Request<Body>;
        type Response = Response<Body>;

        #[allow(unused_variables)]
        fn spawn(
            future: impl Future<Output = ()> + Send + 'static,
        ) -> Result<(), Error> {
            #[cfg(feature = "axum")]
            {
                tokio::spawn(future);
                Ok(())
            }
            #[cfg(not(feature = "axum"))]
            {
                Err(Error::from_server_fn_error(
                    crate::error::ServerFnErrorErr::Request(
                        "No async runtime available. You need to either \
                         enable the full axum feature to pull in tokio, or \
                         implement the `Server` trait for your async runtime \
                         manually."
                            .into(),
                    ),
                ))
            }
        }
    }

    /// Explicitly register a server function. This is only necessary if you are
    /// running the server in a WASM environment (or a rare environment that the
    /// `inventory` crate won't work in.).
    pub fn register_explicit<T>()
    where
        T: ServerFn<
                Server: crate::Server<
                    T::Error,
                    T::InputStreamError,
                    T::OutputStreamError,
                    Request = Request<Body>,
                    Response = Response<Body>,
                >,
            > + 'static,
    {
        REGISTERED_SERVER_FUNCTIONS.insert(
            (T::PATH.into(), T::Protocol::METHOD),
            ServerFnTraitObj::new::<T>(|req| Box::pin(T::run_on_server(req))),
        );
    }

    /// The set of all registered server function paths.
    pub fn server_fn_paths() -> impl Iterator<Item = (&'static str, Method)> {
        REGISTERED_SERVER_FUNCTIONS
            .iter()
            .map(|item| (item.path(), item.method()))
    }

    /// An Axum handler that responds to a server function request.
    pub async fn handle_server_fn(req: Request<Body>) -> Response<Body> {
        let path = req.uri().path();

        if let Some(mut service) =
            get_server_fn_service(path, req.method().clone())
        {
            service.run(req).await
        } else {
            Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(format!(
                    "Could not find a server function at the route {path}. \
                     \n\nIt's likely that either\n 1. The API prefix you \
                     specify in the `#[server]` macro doesn't match the \
                     prefix at which your server function handler is mounted, \
                     or \n2. You are on a platform that doesn't support \
                     automatic server function registration and you need to \
                     call ServerFn::register_explicit() on the server \
                     function type, somewhere in your `main` function.",
                )))
                .unwrap()
        }
    }

    /// Returns the server function at the given path as a service that can be modified.
    pub fn get_server_fn_service(
        path: &str,
        method: Method,
    ) -> Option<BoxedService<Request<Body>, Response<Body>>> {
        let key = (path.into(), method);
        REGISTERED_SERVER_FUNCTIONS.get(&key).map(|server_fn| {
            let middleware = (server_fn.middleware)();
            let mut service = server_fn.clone().boxed();
            for middleware in middleware {
                service = middleware.layer(service);
            }
            service
        })
    }
}

/// Actix integration.
#[cfg(feature = "actix-no-default")]
pub mod actix {
    use crate::{
        error::FromServerFnError, middleware::BoxedService,
        request::actix::ActixRequest, response::actix::ActixResponse,
        server::Server, LazyServerFnMap, Protocol, ServerFn, ServerFnTraitObj,
    };
    use actix_web::{web::Payload, HttpRequest, HttpResponse};
    use http::Method;
    #[doc(hidden)]
    pub use send_wrapper::SendWrapper;
    use std::future::Future;

    static REGISTERED_SERVER_FUNCTIONS: LazyServerFnMap<
        ActixRequest,
        ActixResponse,
    > = initialize_server_fn_map!(ActixRequest, ActixResponse);

    /// The actix server function backend
    pub struct ActixServerFnBackend;

    impl<Error, InputStreamError, OutputStreamError>
        Server<Error, InputStreamError, OutputStreamError>
        for ActixServerFnBackend
    where
        Error: FromServerFnError + Send + Sync,
        InputStreamError: FromServerFnError + Send + Sync,
        OutputStreamError: FromServerFnError + Send + Sync,
    {
        type Request = ActixRequest;
        type Response = ActixResponse;

        fn spawn(
            future: impl Future<Output = ()> + Send + 'static,
        ) -> Result<(), Error> {
            actix_web::rt::spawn(future);
            Ok(())
        }
    }

    /// Explicitly register a server function. This is only necessary if you are
    /// running the server in a WASM environment (or a rare environment that the
    /// `inventory` crate won't work in.).
    pub fn register_explicit<T>()
    where
        T: ServerFn<
                Server: crate::Server<
                    T::Error,
                    T::InputStreamError,
                    T::OutputStreamError,
                    Request = ActixRequest,
                    Response = ActixResponse,
                >,
            > + 'static,
    {
        REGISTERED_SERVER_FUNCTIONS.insert(
            (T::PATH.into(), T::Protocol::METHOD),
            ServerFnTraitObj::new::<T>(|req| Box::pin(T::run_on_server(req))),
        );
    }

    /// The set of all registered server function paths.
    pub fn server_fn_paths() -> impl Iterator<Item = (&'static str, Method)> {
        REGISTERED_SERVER_FUNCTIONS
            .iter()
            .map(|item| (item.path(), item.method()))
    }

    /// An Actix handler that responds to a server function request.
    pub async fn handle_server_fn(
        req: HttpRequest,
        payload: Payload,
    ) -> HttpResponse {
        let path = req.uri().path();
        let method = req.method();
        if let Some(mut service) = get_server_fn_service(path, method) {
            service
                .run(ActixRequest::from((req, payload)))
                .await
                .0
                .take()
        } else {
            HttpResponse::BadRequest().body(format!(
                "Could not find a server function at the route {path}. \
                 \n\nIt's likely that either\n 1. The API prefix you specify \
                 in the `#[server]` macro doesn't match the prefix at which \
                 your server function handler is mounted, or \n2. You are on \
                 a platform that doesn't support automatic server function \
                 registration and you need to call \
                 ServerFn::register_explicit() on the server function type, \
                 somewhere in your `main` function.",
            ))
        }
    }

    /// Returns the server function at the given path as a service that can be modified.
    pub fn get_server_fn_service(
        path: &str,
        method: &actix_web::http::Method,
    ) -> Option<BoxedService<ActixRequest, ActixResponse>> {
        use actix_web::http::Method as ActixMethod;

        let method = match *method {
            ActixMethod::GET => Method::GET,
            ActixMethod::POST => Method::POST,
            ActixMethod::PUT => Method::PUT,
            ActixMethod::PATCH => Method::PATCH,
            ActixMethod::DELETE => Method::DELETE,
            ActixMethod::HEAD => Method::HEAD,
            ActixMethod::TRACE => Method::TRACE,
            ActixMethod::OPTIONS => Method::OPTIONS,
            ActixMethod::CONNECT => Method::CONNECT,
            _ => unreachable!(),
        };
        REGISTERED_SERVER_FUNCTIONS.get(&(path.into(), method)).map(
            |server_fn| {
                let middleware = (server_fn.middleware)();
                let mut service = server_fn.clone().boxed();
                for middleware in middleware {
                    service = middleware.layer(service);
                }
                service
            },
        )
    }
}

/// Mocks for the server function backend types when compiling for the client.
pub mod mock {
    use std::future::Future;

    /// A mocked server type that can be used in place of the actual server,
    /// when compiling for the browser.
    ///
    /// ## Panics
    /// This always panics if its methods are called. It is used solely to stub out the
    /// server type when compiling for the client.
    pub struct BrowserMockServer;

    impl<Error, InputStreamError, OutputStreamError>
        crate::server::Server<Error, InputStreamError, OutputStreamError>
        for BrowserMockServer
    where
        Error: Send + 'static,
        InputStreamError: Send + 'static,
        OutputStreamError: Send + 'static,
    {
        type Request = crate::request::BrowserMockReq;
        type Response = crate::response::BrowserMockRes;

        fn spawn(
            _: impl Future<Output = ()> + Send + 'static,
        ) -> Result<(), Error> {
            unreachable!()
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::codec::JsonEncoding;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    enum TestError {
        ServerFnError(ServerFnErrorErr),
    }

    impl FromServerFnError for TestError {
        type Encoder = JsonEncoding;

        fn from_server_fn_error(value: ServerFnErrorErr) -> Self {
            Self::ServerFnError(value)
        }
    }
    #[test]
    fn test_result_serialization() {
        // Test Ok variant
        let ok_result: Result<Bytes, Bytes> =
            Ok(Bytes::from_static(b"success data"));
        let serialized = serialize_result(ok_result);
        let deserialized = deserialize_result::<TestError>(serialized);
        assert!(deserialized.is_ok());
        assert_eq!(deserialized.unwrap(), Bytes::from_static(b"success data"));

        // Test Err variant
        let err_result: Result<Bytes, Bytes> =
            Err(Bytes::from_static(b"error details"));
        let serialized = serialize_result(err_result);
        let deserialized = deserialize_result::<TestError>(serialized);
        assert!(deserialized.is_err());
        assert_eq!(
            deserialized.unwrap_err(),
            Bytes::from_static(b"error details")
        );
    }
}
