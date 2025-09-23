#![cfg(feature = "ws")]

use axum::extract::{FromRequest, Request};
use axum_core::response::{IntoResponse, Response};
use bytes::Bytes;
use futures::StreamExt;
use http::HeaderValue;
use send_wrapper::SendWrapper;

use crate::{
    websocket::{protocol::CloseCode, wasm::WebSysWebSocketStream},
    CborEncoding, Encoding, FromResponse, IntoRequest, JsonEncoding, ServerFnError,
};
use dioxus_core::{use_hook, CapturedError, RenderError, Result};
use dioxus_hooks::{use_loader, use_memo, use_signal, use_waker, Loader};
use dioxus_hooks::{use_resource, Resource};
use dioxus_signals::{CopyValue, ReadSignal, ReadableOptionExt, Signal, WritableExt};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::TryFutureExt;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Mutex;
use std::{marker::PhantomData, prelude::rust_2024::Future, sync::Arc};

/// A hook that provides a reactive interface to a WebSocket connection.
pub fn use_websocket<
    In: 'static,
    Out: 'static,
    E: Into<dioxus_core::Error> + 'static,
    F: Future<Output = Result<Websocket<In, Out, Enc>, E>> + 'static,
    Enc: Encoding,
>(
    mut connect_to_websocket: impl FnMut() -> F + 'static,
) -> UseWebsocket<In, Out, Enc> {
    let mut status = use_signal(|| WebsocketState::Connecting);
    let status_read = use_hook(|| ReadSignal::new(status));

    let mut waker = use_waker::<()>();

    let connection = use_resource(move || {
        let fut = connect_to_websocket().map_err(|e| CapturedError::from(e.into()));

        async move {
            let res = fut.await;

            // Update the status based on the result of the connection attempt
            match res.as_ref() {
                Ok(_) => {
                    status.set(WebsocketState::Open);
                }
                Err(_) => {
                    status.set(WebsocketState::FailedToConnect);
                }
            }

            // Wake up the receivers
            waker.wake(());

            res
        }
    });

    UseWebsocket {
        _in: PhantomData,
        _out: PhantomData,
        _enc: PhantomData,
        connection,
        waker,
        status,
        status_read,
    }
}

pub struct UseWebsocket<In: 'static, Out: 'static, Enc: 'static = JsonEncoding> {
    _in: std::marker::PhantomData<fn() -> In>,
    _out: std::marker::PhantomData<fn() -> Out>,
    _enc: std::marker::PhantomData<fn() -> Enc>,
    connection: Resource<Result<Websocket<In, Out, Enc>, CapturedError>>,
    waker: dioxus_hooks::UseWaker<()>,
    status: Signal<WebsocketState>,
    status_read: ReadSignal<WebsocketState>,
}

impl<In, Out, E> UseWebsocket<In, Out, E> {}

impl<In, Out, E> Clone for UseWebsocket<In, Out, E> {
    fn clone(&self) -> Self {
        todo!()
    }
}
impl<In, Out, E> Copy for UseWebsocket<In, Out, E> {}

impl<In, Out, E> UseWebsocket<In, Out, E> {
    /// Explicitly wait for the WebSocket connection to be established.
    pub fn connect(&mut self) -> impl Future<Output = Result<(), WebsocketError>> {
        async move { todo!() }
    }

    pub fn connecting(&self) -> bool {
        todo!()
    }

    pub async fn send_raw(&mut self, msg: Bytes) -> Result<(), WebsocketError> {
        todo!()
    }

    pub fn status(&self) -> ReadSignal<WebsocketState> {
        self.status_read
    }

    pub async fn recv_raw(&mut self) -> Result<protocol::Message, WebsocketError> {
        self.wait_for_connection().await;

        let conn = self.connection.value();
        let con_ref = conn.as_ref();
        let connection: Result<&Websocket<In, Out, E>, &CapturedError> = con_ref
            .as_deref()
            .ok_or_else(|| WebsocketError::ConnectionClosed {
                description: "Connection closed".into(),
                reason: CloseCode::Away,
            })?
            .as_ref();

        loop {
            let conn = connection.unwrap();
            let msg = {
                let mut socket = conn.inner_web.as_ref().unwrap().rx.lock().await;
                socket.next().await
            };

            let msg = match msg {
                Some(Some(Ok(msg))) => msg,
                Some(Some(Err(e))) => {
                    todo!()
                }
                Some(None) => {
                    return Err(WebsocketError::ConnectionClosed {
                        description: "Connection closed".into(),
                        reason: CloseCode::Away,
                    })
                }
                None => todo!(),
            };

            match msg {
                protocol::Message::Text(msg) => {
                    return Ok(protocol::Message::Text(msg));
                }
                protocol::Message::Binary(bytes) => {
                    return Ok(protocol::Message::Binary(bytes));
                }
                protocol::Message::Ping(bytes) => todo!("Respond with a pong"),
                protocol::Message::Pong(bytes) => continue,
                protocol::Message::Close { code, reason } => {
                    self.status.set(WebsocketState::Closed);
                    return Err(WebsocketError::ConnectionClosed {
                        description: reason,
                        reason: code,
                    });
                }
            }
        }
    }

    /// Wait for the connection to be established. This guarantees that subsequent calls to methods like
    /// `.try_recv()` will not fail due to the connection not being ready.
    pub async fn wait_for_connection(&mut self) {
        // Wait for the connection to be established
        while !self.connection.finished() {
            _ = self.waker.wait().await;
        }
    }
}

impl<In: Serialize, Out, E: Encoding> UseWebsocket<In, Out, E> {
    pub async fn send(&self, msg: In) -> Result<(), WebsocketError> {
        // Wait for the connection to be established
        while !self.connection.finished() {
            _ = self.waker.wait().await;
        }

        let bytes = E::to_bytes(&msg);

        let conn = self.connection.value();
        let con_ref = conn.as_ref();
        let connection: Result<&Websocket<In, Out, E>, &CapturedError> = con_ref
            .as_deref()
            .ok_or_else(|| WebsocketError::ConnectionClosed {
                description: "Connection closed".into(),
                reason: CloseCode::Away,
            })?
            .as_ref();

        let r = connection.unwrap().inner_web.as_ref().unwrap();
        r.inner.send_with_u8_array(&bytes.unwrap()).unwrap();

        Ok(())
    }
}

impl<In, Out: DeserializeOwned, E: Encoding> UseWebsocket<In, Out, E> {
    /// Receive the next message from the WebSocket connection, deserialized into the `Out` type.
    ///
    /// If the connection is still opening, this will wait until the connection is established.
    /// If the connection fails to open or is killed while waiting, an error will be returned.
    ///
    /// This method returns an error if the connection is closed since we assume closed connections
    /// are a "failure".
    pub async fn recv(&mut self) -> Result<Out, WebsocketError> {
        let msg = self.recv_raw().await?;

        match msg {
            protocol::Message::Text(text) => {
                let res: Out = E::from_bytes(text.into()).ok_or_else(|| {
                    WebsocketError::Deserialization(
                        anyhow::anyhow!("Failed to deserialize text message").into(),
                    )
                })?;
                Ok(res)
            }
            protocol::Message::Binary(bytes) => {
                let res: Out = E::from_bytes(bytes.into()).ok_or_else(|| {
                    WebsocketError::Deserialization(
                        anyhow::anyhow!("Failed to deserialize binary message").into(),
                    )
                })?;
                Ok(res)
            }
            protocol::Message::Ping(bytes) => todo!(),
            protocol::Message::Pong(bytes) => todo!(),
            protocol::Message::Close { code, reason } => todo!(),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum WebsocketError {
    #[error("Connection closed")]
    ConnectionClosed {
        reason: CloseCode,
        description: String,
    },

    #[error("WebSocket already closed")]
    AlreadyClosed,

    #[error("WebSocket capacity reached")]
    Capacity,

    #[cfg(not(target_arch = "wasm32"))]
    #[cfg_attr(docsrs, doc(cfg(not(target_arch = "wasm32"))))]
    #[error("websocket upgrade failed")]
    Handshake(#[from] native::HandshakeError),

    #[error("reqwest error")]
    Reqwest(#[from] reqwest::Error),

    #[cfg(not(target_arch = "wasm32"))]
    #[cfg_attr(docsrs, doc(cfg(not(target_arch = "wasm32"))))]
    #[error("tungstenite error")]
    Tungstenite(#[from] tungstenite::Error),

    #[cfg(target_arch = "wasm32")]
    #[cfg_attr(docsrs, doc(cfg(target_arch = "wasm32")))]
    #[error("web_sys error")]
    WebSys(#[from] wasm::WebSysError),

    /// Error during serialization/deserialization.
    #[error("error during serialization/deserialization")]
    Deserialization(Box<dyn std::error::Error + Send + Sync>),

    /// Error during serialization/deserialization.
    #[error("error during serialization/deserialization")]
    Serialization(Box<dyn std::error::Error + Send + Sync>),

    /// Error during serialization/deserialization.
    #[error("serde_json error")]
    #[cfg_attr(docsrs, doc(cfg(feature = "json")))]
    Json(#[from] serde_json::Error),

    /// Error during serialization/deserialization.
    #[error("ciborium error")]
    Cbor(#[from] ciborium::de::Error<std::io::Error>),
}

#[derive(Debug)]
pub enum WebsocketState {
    /// The WebSocket is connecting.
    Connecting,

    /// The WebSocket is open and ready to send and receive messages.
    Open,

    /// The WebSocket is closing.
    Closing,

    /// The WebSocket is closed and cannot be used to send or receive messages.
    Closed,

    /// The WebSocket failed to connect
    FailedToConnect,
}

impl<In: Serialize, Out: DeserializeOwned> Websocket<In, Out> {
    pub async fn send(&self, msg: In) -> Result<(), ServerFnError> {
        todo!()
    }

    pub async fn recv(&mut self) -> Result<Out, ServerFnError> {
        todo!()
    }
}

// Create a new WebSocket connection that uses the provided function to handle incoming messages
impl<In, Out, E> IntoResponse for Websocket<In, Out, E> {
    fn into_response(self) -> Response {
        self.response.unwrap().into_response()
    }
}

/// A WebSocket connection that can send and receive messages of type `In` and `Out`.
pub struct Websocket<In = String, Out = String, E = JsonEncoding> {
    _in: std::marker::PhantomData<fn() -> In>,
    _out: std::marker::PhantomData<fn() -> Out>,
    _enc: std::marker::PhantomData<fn() -> E>,

    // #[cfg(not(target_arch = "wasm32"))]
    // inner_native: native::WebSocketStream,

    // #[cfg(target_arch = "wasm32")]
    inner_web: Option<wasm::WebSysWebSocketStream>,

    response: Option<axum::response::Response>,
}

impl<I, O, E> PartialEq for Websocket<I, O, E> {
    fn eq(&self, other: &Self) -> bool {
        todo!()
    }
}

impl<I, O, E> FromResponse<ActiveWebSocketConnection> for Websocket<I, O, E> {
    fn from_response(
        res: ActiveWebSocketConnection,
    ) -> impl Future<Output = Result<Self, ServerFnError>> + Send {
        SendWrapper::new(async move {
            // #[cfg(feature = "web")]
            let inner_web = {
                res.inner
                // todo!()
                // let state = res
                //     .state
                //     .take()
                //     .unwrap()
                //     .downcast::<ActiveWebSocketConnection>()
                //     .unwrap();

                // state.inner.unwrap()
            };

            // #[cfg(not(target_arch = "wasm32"))]
            // let (inner, protocol) = res
            //     .inner
            //     .into_stream_and_protocol(res.protocols, res.web_socket_config)
            //     .await?;

            // WebSysWebSocketStream::new(, protocols)
            // // #[cfg(target_arch = "wasm32")]
            // let (inner, protocol) = {
            //     let protocol = res.inner.protocol();
            //     (res.inner, Some(protocol))
            // };

            Ok(Websocket {
                inner_web: Some(inner_web),
                response: None,
                _in: PhantomData,
                _out: PhantomData,
                _enc: PhantomData,
            })
        })
    }
}

pub struct WebSocketOptions {
    _private: (),
    protocols: Vec<String>,
    #[cfg(feature = "server")]
    upgrade: Option<axum::extract::ws::WebSocketUpgrade>,
}

impl WebSocketOptions {
    pub fn new() -> Self {
        Self {
            _private: (),
            protocols: Vec::new(),

            #[cfg(feature = "server")]
            upgrade: None,
        }
    }

    #[cfg(feature = "server")]
    pub fn on_upgrade<F, Fut, In, Out, Enc>(self, callback: F) -> Websocket<In, Out, Enc>
    where
        F: FnOnce(TypedWebsocket<In, Out, Enc>) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        let response = self.upgrade.unwrap().on_upgrade(|socket| {
            let res = crate::spawn_platform(move || {
                callback(TypedWebsocket {
                    _in: PhantomData,
                    _out: PhantomData,
                    _enc: PhantomData,
                    inner: socket,
                })
            });
            async move {
                let _ = res.await;
            }
        });

        Websocket {
            response: Some(response),
            _in: PhantomData,
            _out: PhantomData,
            _enc: PhantomData,
            inner_web: None,
        }
    }
}

#[cfg(feature = "server")]
pub struct TypedWebsocket<In, Out, E = JsonEncoding> {
    _in: std::marker::PhantomData<fn() -> In>,
    _out: std::marker::PhantomData<fn() -> Out>,
    _enc: std::marker::PhantomData<fn() -> E>,

    inner: axum::extract::ws::WebSocket,
}

#[cfg(feature = "server")]
impl<In: DeserializeOwned, Out: Serialize, E: Encoding> TypedWebsocket<In, Out, E> {
    /// Receive another message.
    ///
    /// Returns `None` if the stream has closed.
    pub async fn recv(&mut self) -> Option<Result<In, WebsocketError>> {
        let res = self.inner.next().await?;
        match res {
            Ok(res) => {
                let e: In = E::from_bytes(res.into_data()).unwrap();
                return Some(Ok(e));
            }
            Err(res) => return todo!(),
        }
    }

    /// Send a message.
    pub async fn send(&mut self, msg: Out) -> Result<(), WebsocketError> {
        use axum::extract::ws::Message;

        let to_bytes = E::to_bytes(&msg).ok_or_else(|| {
            WebsocketError::Serialization(anyhow::anyhow!("Failed to serialize message").into())
        })?;

        let res = self
            .inner
            .send(Message::Binary(to_bytes.into()))
            .await
            .unwrap();

        Ok(())
    }

    /// Receive another message.
    ///
    /// Returns `None` if the stream has closed.
    pub async fn recv_raw(&mut self) -> Option<Result<Out, WebsocketError>> {
        // let res = self.inner.next().await;
        todo!()
    }

    /// Send a message.
    pub async fn send_raw(
        &mut self,
        msg: axum::extract::ws::Message,
    ) -> Result<(), WebsocketError> {
        todo!()
        // self.inner
        //     .send(msg.into_tungstenite())
        //     .await
        //     .map_err(Error::new)
    }

    /// Return the selected WebSocket subprotocol, if one has been chosen.
    pub fn protocol(&self) -> Option<&HeaderValue> {
        // self.protocol.as_ref()
        todo!()
    }
}

pub struct ActiveWebSocketConnection {
    inner: wasm::WebSysWebSocketStream,
}
unsafe impl Send for ActiveWebSocketConnection {}
unsafe impl Sync for ActiveWebSocketConnection {}

impl IntoRequest<ActiveWebSocketConnection> for WebSocketOptions {
    fn into_request(
        self,
        builder: reqwest::RequestBuilder,
    ) -> impl Future<Output = std::result::Result<ActiveWebSocketConnection, reqwest::Error>>
           + Send
           + 'static {
        send_wrapper::SendWrapper::new(async move {
            let inner = wasm::WebSysWebSocketStream::new(builder.build()?, &self.protocols)
                .await
                .unwrap();

            Ok(ActiveWebSocketConnection { inner })
        })
    }
}

impl<S: Send> FromRequest<S> for WebSocketOptions {
    type Rejection = axum::http::StatusCode;

    fn from_request(
        req: Request,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        #[cfg(not(feature = "server"))]
        return async move { Err(axum::http::StatusCode::NOT_IMPLEMENTED) };

        #[cfg(feature = "server")]
        async move {
            let ws = match axum::extract::ws::WebSocketUpgrade::from_request(req, &()).await {
                Ok(ws) => ws,
                Err(rejection) => todo!(),
            };

            Ok(WebSocketOptions {
                _private: (),
                protocols: vec![],
                upgrade: Some(ws),
            })
        }
    }
}

// #[cfg(target_arch = "wasm32")]
#[cfg(feature = "web")]
mod wasm {
    use std::{
        pin::Pin,
        task::{Context, Poll},
    };

    use futures::{FutureExt, StreamExt};
    use futures_channel::{mpsc, oneshot};
    use futures_util::{Sink, Stream};
    use reqwest::{Request, Url};
    use web_sys::{
        js_sys::{Array, ArrayBuffer, JsString, Uint8Array},
        wasm_bindgen::{closure::Closure, JsCast, JsValue},
        CloseEvent, ErrorEvent, Event, MessageEvent,
    };

    use super::protocol::{CloseCode, Message};

    #[derive(Debug, thiserror::Error)]
    pub enum WebSysError {
        #[error("invalid url: {0}")]
        InvalidUrl(Url),

        #[error("connection failed")]
        ConnectionFailed,

        #[error("{0}")]
        ErrorEvent(String),

        #[error("unknown error")]
        Unknown,
    }

    impl From<ErrorEvent> for WebSysError {
        fn from(event: ErrorEvent) -> Self {
            Self::ErrorEvent(event.message())
        }
    }

    impl From<JsValue> for WebSysError {
        fn from(_value: JsValue) -> Self {
            Self::Unknown
        }
    }

    #[derive(Debug)]
    pub struct WebSysWebSocketStream {
        pub(crate) inner: web_sys::WebSocket,

        pub(crate) rx: futures_util::lock::Mutex<
            mpsc::UnboundedReceiver<Option<Result<Message, WebSysError>>>,
        >,

        #[allow(dead_code)]
        on_message_callback: Closure<dyn FnMut(MessageEvent)>,

        #[allow(dead_code)]
        on_error_callback: Closure<dyn FnMut(Event)>,

        #[allow(dead_code)]
        on_close_callback: Closure<dyn FnMut(CloseEvent)>,
    }

    impl WebSysWebSocketStream {
        pub async fn new(request: Request, protocols: &[String]) -> Result<Self, WebSysError> {
            let mut url = request.url().clone();
            let scheme = match url.scheme() {
                "http" | "ws" => "ws",
                "https" | "wss" => "wss",
                _ => return Err(WebSysError::InvalidUrl(url)),
            };

            if url.set_scheme(scheme).is_err() {
                return Err(WebSysError::InvalidUrl(url));
            }

            // the channel for messages and errors
            let (tx, rx) = mpsc::unbounded();

            // channel to signal when the websocket has been opened
            let (open_success_tx, mut open_success_rx) = oneshot::channel();
            let mut open_success_tx = Some(open_success_tx);

            // channel to signal an error while opening the channel
            let (open_error_tx, mut open_error_rx) = oneshot::channel();
            let mut open_error_tx = Some(open_error_tx);

            // create websocket
            let inner = web_sys::WebSocket::new_with_str_sequence(
                url.as_ref(),
                &protocols
                    .iter()
                    .map(|s| JsString::from(s.to_owned()))
                    .collect::<Array>(),
            )
            .map_err(|_| WebSysError::ConnectionFailed)?;

            inner.set_binary_type(web_sys::BinaryType::Arraybuffer);

            // register message handler
            let on_message_callback = {
                let tx = tx.clone();
                Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
                    tracing::debug!(event = ?event.data(), "message event");

                    if let Ok(abuf) = event.data().dyn_into::<ArrayBuffer>() {
                        let array = Uint8Array::new(&abuf);
                        let data = array.to_vec();
                        let _ = tx.unbounded_send(Some(Ok(Message::Binary(data.into()))));
                    } else if let Ok(text) = event.data().dyn_into::<JsString>() {
                        let _ = tx.unbounded_send(Some(Ok(Message::Text(text.into()))));
                    } else {
                        tracing::debug!(event = ?event.data(), "received unknown message event");
                    }
                })
            };
            inner.set_onmessage(Some(on_message_callback.as_ref().unchecked_ref()));

            // register error handler
            // this will try to put the first error into a oneshot channel for errors that
            // happen during opening. once that has been used, or the oneshot
            // channel is dropped, this uses the regular message channel
            let on_error_callback = {
                let tx = tx.clone();
                Closure::<dyn FnMut(Event)>::new(move |event: Event| {
                    let error = match event.dyn_into::<ErrorEvent>() {
                        Ok(error) => WebSysError::from(error),
                        Err(_event) => {
                            tracing::debug!(event = ?_event, "received unknown error event");
                            WebSysError::Unknown
                        }
                    };
                    tracing::debug!("received error event: {error}");

                    let error = if let Some(open_error_tx) = open_error_tx.take() {
                        match open_error_tx.send(error) {
                            Ok(()) => return,
                            Err(error) => error,
                        }
                    } else {
                        error
                    };

                    let _ = tx.unbounded_send(Some(Err(error)));
                })
            };
            inner.set_onerror(Some(on_error_callback.as_ref().unchecked_ref()));

            // register close handler
            let on_close_callback = {
                let tx = tx.clone();
                Closure::<dyn FnMut(CloseEvent)>::new(move |event: CloseEvent| {
                    tracing::debug!("received close event");

                    let _ = tx.unbounded_send(Some(Ok(Message::Close {
                        code: event.code().into(),
                        reason: event.reason(),
                    })));
                    let _ = tx.unbounded_send(None);
                })
            };
            inner.set_onclose(Some(on_close_callback.as_ref().unchecked_ref()));

            // register open handler
            let on_open_callback = Closure::<dyn FnMut(Event)>::new(move |_event: Event| {
                tracing::debug!("received open event");
                if let Some(tx) = open_success_tx.take() {
                    let _ = tx.send(());
                }
            });
            inner.set_onopen(Some(on_open_callback.as_ref().unchecked_ref()));

            // wait for either the open event or an error
            futures::select! {
                res = open_success_rx.fuse() => {},
                res = open_error_rx.fuse() => {
                    if let Ok(err) = res {
                        // cleanup
                        let _result = inner.close();
                        inner.set_onopen(None);
                        inner.set_onmessage(None);
                        inner.set_onclose(None);
                        inner.set_onerror(None);
                        return Err(err);
                    }
                },
            };

            // remove open handler
            inner.set_onopen(None);

            let rx = futures_util::lock::Mutex::new(rx);

            Ok(Self {
                inner,
                on_message_callback,
                on_error_callback,
                on_close_callback,
                rx,
            })
        }

        pub fn protocol(&self) -> String {
            self.inner.protocol()
        }

        pub fn close(self, code: CloseCode, reason: &str) -> Result<(), WebSysError> {
            self.inner.close_with_code_and_reason(code.into(), reason)?;
            Ok(())
        }
    }

    impl Drop for WebSysWebSocketStream {
        fn drop(&mut self) {
            tracing::debug!("websocket stream dropped");
            let _result = self.inner.close();
            self.inner.set_onmessage(None);
            self.inner.set_onclose(None);
            self.inner.set_onerror(None);
        }
    }

    impl Stream for WebSysWebSocketStream {
        type Item = Result<Message, WebSysError>;

        fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            self.rx
                .get_mut()
                .poll_next_unpin(cx)
                .map(|ready_value| ready_value.flatten())
        }
    }

    impl Sink<Message> for WebSysWebSocketStream {
        type Error = WebSysError;

        fn poll_ready(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn start_send(self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
            match item {
                Message::Text(text) => self.inner.send_with_str(&text)?,
                Message::Binary(data) => self.inner.send_with_u8_array(&data)?,
                Message::Close { code, reason } => self
                    .inner
                    .close_with_code_and_reason(code.into(), &reason)?,
                #[allow(deprecated)]
                Message::Ping(_) | Message::Pong(_) => {
                    // ignored!
                }
            }
            Ok(())
        }

        fn poll_flush(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn poll_close(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(self.inner.close().map_err(Into::into))
        }
    }
}

mod protocol {
    use bytes::Bytes;
    use serde::{de::DeserializeOwned, Serialize};

    use crate::WebsocketError;

    /// A `WebSocket` message, which can be a text string or binary data.
    #[derive(Clone, Debug)]
    pub enum Message {
        /// A text `WebSocket` message.
        // note: we can't use `tungstenite::Utf8String` here, since we don't have tungstenite on wasm.
        Text(String),

        /// A binary `WebSocket` message.
        Binary(Bytes),

        /// A ping message with the specified payload.
        ///
        /// The payload here must have a length less than 125 bytes.
        ///
        /// # WASM
        ///
        /// This variant is ignored for WASM targets.
        #[cfg_attr(
            target_arch = "wasm32",
            deprecated(note = "This variant is ignored for WASM targets")
        )]
        Ping(Bytes),

        /// A pong message with the specified payload.
        ///
        /// The payload here must have a length less than 125 bytes.
        ///
        /// # WASM
        ///
        /// This variant is ignored for WASM targets.
        #[cfg_attr(
            target_arch = "wasm32",
            deprecated(note = "This variant is ignored for WASM targets")
        )]
        Pong(Bytes),

        /// A close message.
        ///
        /// Sending this will not close the connection. Use [`WebSocket::close`] for this.
        /// Though the remote peer will likely close the connection after receiving this.
        ///
        /// [`WebSocket::close`]: crate::WebSocket::close
        Close { code: CloseCode, reason: String },
    }

    impl From<String> for Message {
        #[inline]
        fn from(value: String) -> Self {
            Self::Text(value)
        }
    }

    impl From<&str> for Message {
        #[inline]
        fn from(value: &str) -> Self {
            Self::from(value.to_owned())
        }
    }

    impl From<Bytes> for Message {
        #[inline]
        fn from(value: Bytes) -> Self {
            Self::Binary(value)
        }
    }

    impl From<Vec<u8>> for Message {
        #[inline]
        fn from(value: Vec<u8>) -> Self {
            Self::from(Bytes::from(value))
        }
    }

    impl From<&[u8]> for Message {
        #[inline]
        fn from(value: &[u8]) -> Self {
            Self::from(Bytes::copy_from_slice(value))
        }
    }

    /// Status code used to indicate why an endpoint is closing the `WebSocket`
    /// connection.[1]
    ///
    /// [1]: https://datatracker.ietf.org/doc/html/rfc6455
    #[derive(Debug, Default, Eq, PartialEq, Clone, Copy)]
    #[non_exhaustive]
    pub enum CloseCode {
        /// Indicates a normal closure, meaning that the purpose for
        /// which the connection was established has been fulfilled.
        #[default]
        Normal,

        /// Indicates that an endpoint is "going away", such as a server
        /// going down or a browser having navigated away from a page.
        Away,

        /// Indicates that an endpoint is terminating the connection due
        /// to a protocol error.
        Protocol,

        /// Indicates that an endpoint is terminating the connection
        /// because it has received a type of data it cannot accept (e.g., an
        /// endpoint that understands only text data MAY send this if it
        /// receives a binary message).
        Unsupported,

        /// Indicates that no status code was included in a closing frame. This
        /// close code makes it possible to use a single method, `on_close` to
        /// handle even cases where no close code was provided.
        Status,

        /// Indicates an abnormal closure. If the abnormal closure was due to an
        /// error, this close code will not be used. Instead, the `on_error` method
        /// of the handler will be called with the error. However, if the connection
        /// is simply dropped, without an error, this close code will be sent to the
        /// handler.
        Abnormal,

        /// Indicates that an endpoint is terminating the connection
        /// because it has received data within a message that was not
        /// consistent with the type of the message (e.g., non-UTF-8 \[RFC3629\]
        /// data within a text message).
        Invalid,

        /// Indicates that an endpoint is terminating the connection
        /// because it has received a message that violates its policy.  This
        /// is a generic status code that can be returned when there is no
        /// other more suitable status code (e.g., Unsupported or Size) or if there
        /// is a need to hide specific details about the policy.
        Policy,

        /// Indicates that an endpoint is terminating the connection
        /// because it has received a message that is too big for it to
        /// process.
        Size,

        /// Indicates that an endpoint (client) is terminating the
        /// connection because it has expected the server to negotiate one or
        /// more extension, but the server didn't return them in the response
        /// message of the `WebSocket` handshake.  The list of extensions that
        /// are needed should be given as the reason for closing.
        /// Note that this status code is not used by the server, because it
        /// can fail the `WebSocket` handshake instead.
        Extension,

        /// Indicates that a server is terminating the connection because
        /// it encountered an unexpected condition that prevented it from
        /// fulfilling the request.
        Error,

        /// Indicates that the server is restarting. A client may choose to
        /// reconnect, and if it does, it should use a randomized delay of 5-30
        /// seconds between attempts.
        Restart,

        /// Indicates that the server is overloaded and the client should either
        /// connect to a different IP (when multiple targets exist), or
        /// reconnect to the same IP when a user has performed an action.
        Again,

        /// Indicates that the connection was closed due to a failure to perform a
        /// TLS handshake (e.g., the server certificate can't be verified). This
        /// is a reserved value and MUST NOT be set as a status code in a Close
        /// control frame by an endpoint.
        Tls,

        /// Reserved status codes.
        Reserved(u16),

        /// Reserved for use by libraries, frameworks, and applications. These
        /// status codes are registered directly with IANA. The interpretation of
        /// these codes is undefined by the `WebSocket` protocol.
        Iana(u16),

        /// Reserved for private use. These can't be registered and can be used by
        /// prior agreements between `WebSocket` applications. The interpretation of
        /// these codes is undefined by the `WebSocket` protocol.
        Library(u16),

        /// Unused / invalid status codes.
        Bad(u16),
    }

    impl CloseCode {
        /// Check if this `CloseCode` is allowed.
        #[must_use]
        pub const fn is_allowed(self) -> bool {
            !matches!(
                self,
                Self::Bad(_) | Self::Reserved(_) | Self::Status | Self::Abnormal | Self::Tls
            )
        }
    }

    impl std::fmt::Display for CloseCode {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            let code: u16 = (*self).into();
            write!(f, "{code}")
        }
    }

    impl From<CloseCode> for u16 {
        fn from(code: CloseCode) -> Self {
            match code {
                CloseCode::Normal => 1000,
                CloseCode::Away => 1001,
                CloseCode::Protocol => 1002,
                CloseCode::Unsupported => 1003,
                CloseCode::Status => 1005,
                CloseCode::Abnormal => 1006,
                CloseCode::Invalid => 1007,
                CloseCode::Policy => 1008,
                CloseCode::Size => 1009,
                CloseCode::Extension => 1010,
                CloseCode::Error => 1011,
                CloseCode::Restart => 1012,
                CloseCode::Again => 1013,
                CloseCode::Tls => 1015,
                CloseCode::Reserved(code)
                | CloseCode::Iana(code)
                | CloseCode::Library(code)
                | CloseCode::Bad(code) => code,
            }
        }
    }

    impl From<u16> for CloseCode {
        fn from(code: u16) -> Self {
            match code {
                1000 => Self::Normal,
                1001 => Self::Away,
                1002 => Self::Protocol,
                1003 => Self::Unsupported,
                1005 => Self::Status,
                1006 => Self::Abnormal,
                1007 => Self::Invalid,
                1008 => Self::Policy,
                1009 => Self::Size,
                1010 => Self::Extension,
                1011 => Self::Error,
                1012 => Self::Restart,
                1013 => Self::Again,
                1015 => Self::Tls,
                1016..=2999 => Self::Reserved(code),
                3000..=3999 => Self::Iana(code),
                4000..=4999 => Self::Library(code),
                _ => Self::Bad(code),
            }
        }
    }

    impl Message {
        /// Tries to serialize the JSON as a [`Message::Text`].
        ///
        /// # Optional
        ///
        /// This requires the optional `json` feature enabled.
        ///
        /// # Errors
        ///
        /// Serialization can fail if `T`'s implementation of `Serialize` decides to
        /// fail, or if `T` contains a map with non-string keys.
        #[cfg_attr(docsrs, doc(cfg(feature = "json")))]
        pub fn text_from_json<T: Serialize + ?Sized>(json: &T) -> Result<Self, WebsocketError> {
            serde_json::to_string(json)
                .map(Message::Text)
                .map_err(Into::into)
        }

        /// Tries to serialize the JSON as a [`Message::Binary`].
        ///
        /// # Optional
        ///
        /// This requires that the optional `json` feature is enabled.
        ///
        /// # Errors
        ///
        /// Serialization can fail if `T`'s implementation of `Serialize` decides to
        /// fail, or if `T` contains a map with non-string keys.
        #[cfg_attr(docsrs, doc(cfg(feature = "json")))]
        pub fn binary_from_json<T: Serialize + ?Sized>(json: &T) -> Result<Self, WebsocketError> {
            serde_json::to_vec(json)
                .map(|data| Message::Binary(data.into()))
                .map_err(Into::into)
        }

        /// Tries to deserialize the message body as JSON.
        ///
        /// # Optional
        ///
        /// This requires that the optional `json` feature is enabled.
        ///
        /// # Errors
        ///
        /// This method fails whenever the response body is not in `JSON` format,
        /// or it cannot be properly deserialized to target type `T`.
        ///
        /// For more details please see [`serde_json::from_str`] and
        /// [`serde_json::from_slice`].
        #[cfg_attr(docsrs, doc(cfg(feature = "json")))]
        pub fn json<T: DeserializeOwned>(&self) -> Result<T, WebsocketError> {
            use serde::de::Error as _;
            match self {
                Self::Text(x) => serde_json::from_str(x).map_err(Into::into),
                Self::Binary(x) => serde_json::from_slice(x).map_err(Into::into),
                #[allow(deprecated)]
                Self::Ping(_) | Self::Pong(_) | Self::Close { .. } => {
                    Err(serde_json::Error::custom("neither text nor binary").into())
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::borrow::Cow;

    use super::{
        protocol::{CloseCode, Message},
        WebsocketError,
    };
    use reqwest::{
        header::{HeaderName, HeaderValue},
        RequestBuilder, Response, StatusCode, Version,
    };
    use tungstenite::protocol::WebSocketConfig;

    pub async fn send_request(
        request_builder: RequestBuilder,
        protocols: &[String],
    ) -> Result<WebSocketResponse, WebsocketError> {
        let (client, request_result) = request_builder.build_split();
        let mut request = request_result?;

        // change the scheme from wss? to https?
        let url = request.url_mut();
        match url.scheme() {
            "ws" => {
                url.set_scheme("http")
                    .expect("url should accept http scheme");
            }
            "wss" => {
                url.set_scheme("https")
                    .expect("url should accept https scheme");
            }
            _ => {}
        }

        // prepare request
        let version = request.version();
        let nonce = match version {
            Version::HTTP_10 | Version::HTTP_11 => {
                // HTTP 1 requires us to set some headers.
                let nonce_value = tungstenite::handshake::client::generate_key();
                let headers = request.headers_mut();
                headers.insert(
                    reqwest::header::CONNECTION,
                    HeaderValue::from_static("upgrade"),
                );
                headers.insert(
                    reqwest::header::UPGRADE,
                    HeaderValue::from_static("websocket"),
                );
                headers.insert(
                    reqwest::header::SEC_WEBSOCKET_KEY,
                    HeaderValue::from_str(&nonce_value).expect("nonce is a invalid header value"),
                );
                headers.insert(
                    reqwest::header::SEC_WEBSOCKET_VERSION,
                    HeaderValue::from_static("13"),
                );
                if !protocols.is_empty() {
                    headers.insert(
                        reqwest::header::SEC_WEBSOCKET_PROTOCOL,
                        HeaderValue::from_str(&protocols.join(", "))
                            .expect("protocols is an invalid header value"),
                    );
                }

                Some(nonce_value)
            }
            Version::HTTP_2 => {
                // TODO: Implement websocket upgrade for HTTP 2.
                return Err(HandshakeError::UnsupportedHttpVersion(version).into());
            }
            _ => {
                return Err(HandshakeError::UnsupportedHttpVersion(version).into());
            }
        };

        // execute request
        let response = client.execute(request).await?;

        Ok(WebSocketResponse {
            response,
            version,
            nonce,
        })
    }

    pub type WebSocketStream =
        async_tungstenite::WebSocketStream<tokio_util::compat::Compat<reqwest::Upgraded>>;

    /// Error during `Websocket` handshake.
    #[derive(Debug, thiserror::Error, Clone)]
    pub enum HandshakeError {
        #[error("unsupported http version: {0:?}")]
        UnsupportedHttpVersion(Version),

        #[error("the server responded with a different http version. this could be the case because reqwest silently upgraded the connection to http2. see: https://github.com/jgraef/reqwest-websocket/issues/2")]
        ServerRespondedWithDifferentVersion,

        #[error("missing header {header}")]
        MissingHeader { header: HeaderName },

        #[error("unexpected value for header {header}: expected {expected}, but got {got:?}.")]
        UnexpectedHeaderValue {
            header: HeaderName,
            got: HeaderValue,
            expected: Cow<'static, str>,
        },

        #[error("expected the server to select a protocol.")]
        ExpectedAProtocol,

        #[error("unexpected protocol: {got}")]
        UnexpectedProtocol { got: String },

        #[error("unexpected status code: {0}")]
        UnexpectedStatusCode(StatusCode),
    }

    pub struct WebSocketResponse {
        pub response: Response,
        pub version: Version,
        pub nonce: Option<String>,
    }

    impl WebSocketResponse {
        pub async fn into_stream_and_protocol(
            self,
            protocols: Vec<String>,
            web_socket_config: Option<WebSocketConfig>,
        ) -> Result<(WebSocketStream, Option<String>), WebsocketError> {
            let headers = self.response.headers();

            if self.response.version() != self.version {
                return Err(HandshakeError::ServerRespondedWithDifferentVersion.into());
            }

            if self.response.status() != reqwest::StatusCode::SWITCHING_PROTOCOLS {
                tracing::debug!(status_code = %self.response.status(), "server responded with unexpected status code");
                return Err(HandshakeError::UnexpectedStatusCode(self.response.status()).into());
            }

            if let Some(header) = headers.get(reqwest::header::CONNECTION) {
                if !header
                    .to_str()
                    .is_ok_and(|s| s.eq_ignore_ascii_case("upgrade"))
                {
                    tracing::debug!("server responded with invalid Connection header: {header:?}");
                    return Err(HandshakeError::UnexpectedHeaderValue {
                        header: reqwest::header::CONNECTION,
                        got: header.clone(),
                        expected: "upgrade".into(),
                    }
                    .into());
                }
            } else {
                tracing::debug!("missing Connection header");
                return Err(HandshakeError::MissingHeader {
                    header: reqwest::header::CONNECTION,
                }
                .into());
            }

            if let Some(header) = headers.get(reqwest::header::UPGRADE) {
                if !header
                    .to_str()
                    .is_ok_and(|s| s.eq_ignore_ascii_case("websocket"))
                {
                    tracing::debug!("server responded with invalid Upgrade header: {header:?}");
                    return Err(HandshakeError::UnexpectedHeaderValue {
                        header: reqwest::header::UPGRADE,
                        got: header.clone(),
                        expected: "websocket".into(),
                    }
                    .into());
                }
            } else {
                tracing::debug!("missing Upgrade header");
                return Err(HandshakeError::MissingHeader {
                    header: reqwest::header::UPGRADE,
                }
                .into());
            }

            if let Some(nonce) = &self.nonce {
                let expected_nonce = tungstenite::handshake::derive_accept_key(nonce.as_bytes());

                if let Some(header) = headers.get(reqwest::header::SEC_WEBSOCKET_ACCEPT) {
                    if !header.to_str().is_ok_and(|s| s == expected_nonce) {
                        tracing::debug!(
                            "server responded with invalid Sec-Websocket-Accept header: {header:?}"
                        );
                        return Err(HandshakeError::UnexpectedHeaderValue {
                            header: reqwest::header::SEC_WEBSOCKET_ACCEPT,
                            got: header.clone(),
                            expected: expected_nonce.into(),
                        }
                        .into());
                    }
                } else {
                    tracing::debug!("missing Sec-Websocket-Accept header");
                    return Err(HandshakeError::MissingHeader {
                        header: reqwest::header::SEC_WEBSOCKET_ACCEPT,
                    }
                    .into());
                }
            }

            let protocol = headers
                .get(reqwest::header::SEC_WEBSOCKET_PROTOCOL)
                .and_then(|v| v.to_str().ok())
                .map(ToOwned::to_owned);

            match (protocols.is_empty(), &protocol) {
                (true, None) => {
                    // we didn't request any protocols, so we don't expect one
                    // in return
                }
                (false, None) => {
                    // server didn't reply with a protocol
                    return Err(HandshakeError::ExpectedAProtocol.into());
                }
                (false, Some(protocol)) => {
                    if !protocols.contains(protocol) {
                        // the responded protocol is none which we requested
                        return Err(HandshakeError::UnexpectedProtocol {
                            got: protocol.clone(),
                        }
                        .into());
                    }
                }
                (true, Some(protocol)) => {
                    // we didn't request any protocols but got one anyway
                    return Err(HandshakeError::UnexpectedProtocol {
                        got: protocol.clone(),
                    }
                    .into());
                }
            }

            use tokio_util::compat::TokioAsyncReadCompatExt;

            let inner = WebSocketStream::from_raw_socket(
                self.response.upgrade().await?.compat(),
                tungstenite::protocol::Role::Client,
                web_socket_config,
            )
            .await;

            Ok((inner, protocol))
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error("could not convert message")]
    pub struct FromTungsteniteMessageError {
        pub original: tungstenite::Message,
    }

    impl TryFrom<tungstenite::Message> for Message {
        type Error = FromTungsteniteMessageError;

        fn try_from(value: tungstenite::Message) -> Result<Self, Self::Error> {
            match value {
                tungstenite::Message::Text(text) => Ok(Self::Text(text.as_str().to_owned())),
                tungstenite::Message::Binary(data) => Ok(Self::Binary(data)),
                tungstenite::Message::Ping(data) => Ok(Self::Ping(data)),
                tungstenite::Message::Pong(data) => Ok(Self::Pong(data)),
                tungstenite::Message::Close(Some(tungstenite::protocol::CloseFrame {
                    code,
                    reason,
                })) => Ok(Self::Close {
                    code: code.into(),
                    reason: reason.as_str().to_owned(),
                }),
                tungstenite::Message::Close(None) => Ok(Self::Close {
                    code: CloseCode::default(),
                    reason: "".to_owned(),
                }),
                tungstenite::Message::Frame(_) => {
                    Err(FromTungsteniteMessageError { original: value })
                }
            }
        }
    }

    impl From<Message> for tungstenite::Message {
        fn from(value: Message) -> Self {
            match value {
                Message::Text(text) => Self::Text(tungstenite::Utf8Bytes::from(text)),
                Message::Binary(data) => Self::Binary(data),
                Message::Ping(data) => Self::Ping(data),
                Message::Pong(data) => Self::Pong(data),
                Message::Close { code, reason } => {
                    Self::Close(Some(tungstenite::protocol::CloseFrame {
                        code: code.into(),
                        reason: reason.into(),
                    }))
                }
            }
        }
    }

    impl From<tungstenite::protocol::frame::coding::CloseCode> for CloseCode {
        fn from(value: tungstenite::protocol::frame::coding::CloseCode) -> Self {
            u16::from(value).into()
        }
    }

    impl From<CloseCode> for tungstenite::protocol::frame::coding::CloseCode {
        fn from(value: CloseCode) -> Self {
            u16::from(value).into()
        }
    }
}
