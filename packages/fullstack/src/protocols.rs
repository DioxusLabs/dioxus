use base64::{engine::general_purpose::STANDARD_NO_PAD, DecodeError, Engine};

use crate::{
    codec::Codec, ContentType, Decodes, Encodes, FormatType, FromServerFnError, HybridError,
    HybridRequest, HybridResponse, ServerFnError,
};

// use super::client::Client;
use super::codec::Encoding;
// use super::codec::{Encoding, FromReq, FromRes, IntoReq, IntoRes};

// #[cfg(feature = "form-redirects")]
// use super::error::ServerFnUrlError;

use super::middleware::{BoxedService, Layer, Service};
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

/// The protocol that a server function uses to communicate with the client. This trait handles
/// the server and client side of running a server function. It is implemented for the [`Http`] and
/// [`Websocket`] protocols and can be used to implement custom protocols.
pub trait Protocol<Input, Output> {
    /// The HTTP method used for requests.
    const METHOD: Method;

    /// Run the server function on the server. The implementation should handle deserializing the
    /// input, running the server function, and serializing the output.
    fn run_server<F, Fut>(
        request: HybridRequest,
        server_fn: F,
    ) -> impl Future<Output = Result<HybridResponse, HybridError>> + Send
    where
        F: Fn(Input) -> Fut + Send,
        Fut: Future<Output = Result<Output, HybridError>> + Send;

    /// Run the server function on the client. The implementation should handle serializing the
    /// input, sending the request, and deserializing the output.
    fn run_client(
        path: &str,
        input: Input,
    ) -> impl Future<Output = Result<Output, HybridError>> + Send;
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
pub struct Http<InputProtocol, OutputProtocol>(PhantomData<(InputProtocol, OutputProtocol)>);

impl<InputProtocol, OutputProtocol, Input, Output> Protocol<Input, Output>
    for Http<InputProtocol, OutputProtocol>
where
    Input: Codec<InputProtocol>,
    Output: Codec<OutputProtocol>,
    InputProtocol: Encoding,
    OutputProtocol: Encoding,
{
    const METHOD: Method = InputProtocol::METHOD;

    fn run_server<F, Fut>(
        request: HybridRequest,
        server_fn: F,
    ) -> impl Future<Output = Result<HybridResponse, HybridError>> + Send
    where
        F: Fn(Input) -> Fut + Send,
        Fut: Future<Output = Result<Output, HybridError>> + Send,
    {
        async move {
            let input = Input::from_req(request).await?;

            let output = server_fn(input).await?;

            let response = Output::into_res(output).await?;

            Ok(response)
        }
    }

    fn run_client(
        path: &str,
        input: Input,
    ) -> impl Future<Output = Result<Output, HybridError>> + Send {
        async move {
            // create and send request on client
            let req = input.into_req(path, OutputProtocol::CONTENT_TYPE)?;
            let res: HybridResponse = crate::client::current::send(req).await?;

            let status = res.status();
            let location = res.location();
            let has_redirect_header = res.has_redirect();

            // if it returns an error status, deserialize the error using the error's decoder.
            let res = if (400..=599).contains(&status) {
                Err(HybridError::de(res.try_into_bytes().await?))
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
pub struct Websocket<InputEncoding, OutputEncoding>(PhantomData<(InputEncoding, OutputEncoding)>);

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
pub struct BoxedStream<T, E = HybridError> {
    stream: Pin<Box<dyn Stream<Item = Result<T, E>> + Send>>,
}

impl<T, E> From<BoxedStream<T, E>> for Pin<Box<dyn Stream<Item = Result<T, E>> + Send>> {
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

type InputStreamError = HybridError;
type OutputStreamError = HybridError;

impl<
        Input,
        InputItem,
        OutputItem,
        InputEncoding,
        OutputEncoding,
        // Error,
        // InputStreamError,
        // OutputStreamError,
    >
    Protocol<
        Input,
        BoxedStream<OutputItem, OutputStreamError>,
        // Error,
        // InputStreamError,
        // OutputStreamError,
    > for Websocket<InputEncoding, OutputEncoding>
where
    Input: Deref<Target = BoxedStream<InputItem, InputStreamError>>
        + Into<BoxedStream<InputItem, InputStreamError>>
        + From<BoxedStream<InputItem, InputStreamError>>,
    InputEncoding: Encodes<InputItem> + Decodes<InputItem>,
    OutputEncoding: Encodes<OutputItem> + Decodes<OutputItem>,
    // InputStreamError: FromServerFnError + Send,
    // OutputStreamError: FromServerFnError + Send,
    // Error: FromServerFnError + Send,
    OutputItem: Send + 'static,
    InputItem: Send + 'static,
{
    const METHOD: Method = Method::GET;

    async fn run_server<F, Fut>(
        request: HybridRequest,
        server_fn: F,
    ) -> Result<HybridResponse, HybridError>
    where
        F: Fn(Input) -> Fut + Send,
        Fut: Future<Output = Result<BoxedStream<OutputItem, OutputStreamError>, HybridError>>,
    {
        let (request_bytes, response_stream, response) = request.try_into_websocket().await?;
        let input = request_bytes.map(|request_bytes| {
            let request_bytes = request_bytes
                .map(|bytes| crate::deserialize_result::<InputStreamError>(bytes))
                .unwrap_or_else(Err);
            match request_bytes {
                Ok(request_bytes) => InputEncoding::decode(request_bytes).map_err(|e| {
                    InputStreamError::from_server_fn_error(ServerFnError::Deserialization(
                        e.to_string(),
                    ))
                }),
                Err(err) => Err(InputStreamError::de(err)),
            }
        });
        let boxed = Box::pin(input)
            as Pin<Box<dyn Stream<Item = Result<InputItem, InputStreamError>> + Send>>;
        let input = BoxedStream { stream: boxed };

        let output = server_fn(input.into()).await?;

        let output = output.stream.map(|output| {
            let result = match output {
                Ok(output) => OutputEncoding::encode(&output).map_err(|e| {
                    OutputStreamError::from_server_fn_error(ServerFnError::Serialization(
                        e.to_string(),
                    ))
                    .ser()
                }),
                Err(err) => Err(err.ser()),
            };
            crate::serialize_result(result)
        });

        todo!("Spawn a stream");
        // Server::spawn(async move {
        //     pin_mut!(response_stream);
        //     pin_mut!(output);
        //     while let Some(output) = output.next().await {
        //         if response_stream.send(output).await.is_err() {
        //             break;
        //         }
        //     }
        // })?;

        Ok(HybridResponse { res: response })
    }

    fn run_client(
        path: &str,
        input: Input,
    ) -> impl Future<Output = Result<BoxedStream<OutputItem, OutputStreamError>, HybridError>> + Send
    {
        let input = input.into();

        async move {
            todo!()
            // let (stream, sink) = Client::open_websocket(path).await?;

            // // Forward the input stream to the websocket
            // Client::spawn(async move {
            //     pin_mut!(input);
            //     pin_mut!(sink);
            //     while let Some(input) = input.stream.next().await {
            //         let result = match input {
            //             Ok(input) => InputEncoding::encode(&input).map_err(|e| {
            //                 InputStreamError::from_server_fn_error(ServerFnError::Serialization(
            //                     e.to_string(),
            //                 ))
            //                 .ser()
            //             }),
            //             Err(err) => Err(err.ser()),
            //         };
            //         let result = serialize_result(result);
            //         if sink.send(result).await.is_err() {
            //             break;
            //         }
            //     }
            // });

            // // Return the output stream
            // let stream = stream.map(|request_bytes| {
            //     let request_bytes = request_bytes
            //         .map(|bytes| deserialize_result::<OutputStreamError>(bytes))
            //         .unwrap_or_else(Err);
            //     match request_bytes {
            //         Ok(request_bytes) => OutputEncoding::decode(request_bytes).map_err(|e| {
            //             OutputStreamError::from_server_fn_error(ServerFnError::Deserialization(
            //                 e.to_string(),
            //             ))
            //         }),
            //         Err(err) => Err(OutputStreamError::de(err)),
            //     }
            // });
            // let boxed = Box::pin(stream)
            //     as Pin<Box<dyn Stream<Item = Result<OutputItem, OutputStreamError>> + Send>>;
            // let output = BoxedStream { stream: boxed };
            // Ok(output)
        }
    }
}
