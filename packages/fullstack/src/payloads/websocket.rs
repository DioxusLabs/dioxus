#![allow(unreachable_code)]
#![allow(unused_imports)]

//! This module implements WebSocket support for Dioxus Fullstack applications.
//!
//! WebSockets provide a full-duplex communication channel over a single, long-lived connection.
//!
//! This makes them ideal for real-time applications where the server and the client need to communicate
//! frequently and with low latency. Unlike Server-Sent Events (SSE), WebSockets allow the direct
//! transport of binary data, enabling things like video and audio streaming as well as more efficient
//! zero-copy serialization formats.
//!
//! This module implements a variety of types:
//! - `Websocket<In, Out, E>`: Represents a WebSocket connection that can send messages of type `In` and receive messages of type `Out`, using the encoding `E`.
//! - `UseWebsocket<In, Out, E>`: A hook that provides a reactive interface to a WebSocket connection.
//! - `WebSocketOptions`: Configuration options for establishing a WebSocket connection.
//! - `TypedWebsocket<In, Out, E>`: A typed wrapper around an Axum WebSocket connection for server-side use.
//! - `WebsocketState`: An enum representing the state of the WebSocket connection.
//! - plus a variety of error types and traits for encoding/decoding messages.
//!
//! Dioxus Fullstack websockets are typed in both directions, letting the happy path (`.send()` and `.recv()`)
//! automatically serialize and deserialize messages for you.

use crate::{ClientRequest, Encoding, FromResponse, IntoRequest, JsonEncoding, ServerFnError};
use axum::{
    extract::{FromRequest, Request},
    http::StatusCode,
};
use axum_core::response::{IntoResponse, Response};
use bytes::Bytes;
use dioxus_core::{use_hook, CapturedError, Result};
use dioxus_fullstack_core::{HttpError, RequestError};
use dioxus_hooks::{use_resource, Resource, UseWaker};
use dioxus_hooks::{use_signal, use_waker};
use dioxus_signals::{ReadSignal, ReadableExt, ReadableOptionExt, Signal, WritableExt};
use futures::StreamExt;
use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, TryFutureExt,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{marker::PhantomData, prelude::rust_2024::Future};

#[cfg(feature = "web")]
use {
    futures_util::lock::Mutex,
    gloo_net::websocket::{futures::WebSocket as WsWebsocket, Message as WsMessage},
};

/// A hook that provides a reactive interface to a WebSocket connection.
///
/// WebSockets provide a full-duplex communication channel over a single, long-lived connection.
///
/// This makes them ideal for real-time applications where the server and the client need to communicate
/// frequently and with low latency. Unlike Server-Sent Events (SSE), WebSockets allow the direct
/// transport of binary data, enabling things like video and audio streaming as well as more efficient
/// zero-copy serialization formats.
///
/// This hook takes a function that returns a future which resolves to a `Websocket<In, Out, E>` -
/// usually a server function.
pub fn use_websocket<
    In: 'static,
    Out: 'static,
    E: Into<CapturedError> + 'static,
    F: Future<Output = Result<Websocket<In, Out, Enc>, E>> + 'static,
    Enc: Encoding,
>(
    mut connect_to_websocket: impl FnMut() -> F + 'static,
) -> UseWebsocket<In, Out, Enc> {
    let mut waker = use_waker();
    let mut status = use_signal(|| WebsocketState::Connecting);
    let status_read = use_hook(|| ReadSignal::new(status));

    let connection = use_resource(move || {
        let connection = connect_to_websocket().map_err(|e| e.into());
        async move {
            let connection = connection.await;

            // Update the status based on the result of the connection attempt
            match connection.as_ref() {
                Ok(_) => status.set(WebsocketState::Open),
                Err(_) => status.set(WebsocketState::FailedToConnect),
            }

            // Wake up the `.recv()` calls waiting for the connection to be established
            waker.wake(());

            connection
        }
    });

    UseWebsocket {
        connection,
        waker,
        status,
        status_read,
    }
}

/// The return type of the `use_websocket` hook.
///
/// See the `use_websocket` documentation for more details.
///
/// This handle provides methods to send and receive messages, check the connection status,
/// and wait for the connection to be established.
pub struct UseWebsocket<In, Out, Enc = JsonEncoding>
where
    In: 'static,
    Out: 'static,
    Enc: 'static,
{
    connection: Resource<Result<Websocket<In, Out, Enc>, CapturedError>>,
    waker: UseWaker<()>,
    status: Signal<WebsocketState>,
    status_read: ReadSignal<WebsocketState>,
}

impl<In, Out, E> UseWebsocket<In, Out, E> {
    /// Wait for the connection to be established. This guarantees that subsequent calls to methods like
    /// `.try_recv()` will not fail due to the connection not being ready.
    pub async fn connect(&self) -> WebsocketState {
        // Wait for the connection to be established
        while !self.connection.finished() {
            _ = self.waker.wait().await;
        }

        self.status.cloned()
    }

    /// Returns true if the WebSocket is currently connecting.
    ///
    /// This can be useful to present a loading state to the user while the connection is being established.
    pub fn connecting(&self) -> bool {
        matches!(self.status.cloned(), WebsocketState::Connecting)
    }

    /// Returns true if the Websocket is closed due to an error.
    pub fn is_err(&self) -> bool {
        matches!(self.status.cloned(), WebsocketState::FailedToConnect)
    }

    /// Returns true if the WebSocket is currently shut down and cannot be used to send or receive messages.
    pub fn is_closed(&self) -> bool {
        matches!(
            self.status.cloned(),
            WebsocketState::Closed | WebsocketState::FailedToConnect
        )
    }

    /// Get the current status of the WebSocket connection.
    pub fn status(&self) -> ReadSignal<WebsocketState> {
        self.status_read
    }

    /// Send a raw message over the WebSocket connection
    ///
    /// To send a message with a particular type, see the `.send()` method instead.
    pub async fn send_raw(&self, msg: Message) -> Result<(), WebsocketError> {
        self.connect().await;

        self.connection
            .as_ref()
            .as_deref()
            .ok_or_else(WebsocketError::closed_away)?
            .as_ref()
            .map_err(|_| WebsocketError::AlreadyClosed)?
            .send_raw(msg)
            .await
    }

    /// Receive a raw message from the WebSocket connection
    ///
    /// To receive a message with a particular type, see the `.recv()` method instead.
    pub async fn recv_raw(&mut self) -> Result<Message, WebsocketError> {
        self.connect().await;

        let result = self
            .connection
            .as_ref()
            .as_deref()
            .ok_or_else(WebsocketError::closed_away)?
            .as_ref()
            .map_err(|_| WebsocketError::AlreadyClosed)?
            .recv_raw()
            .await;

        if let Err(WebsocketError::ConnectionClosed { .. }) = result.as_ref() {
            self.received_shutdown();
        }

        result
    }

    pub async fn send(&self, msg: In) -> Result<(), WebsocketError>
    where
        In: Serialize,
        E: Encoding,
    {
        self.send_raw(Message::Binary(
            E::to_bytes(&msg).ok_or_else(WebsocketError::serialization)?,
        ))
        .await
    }

    /// Receive the next message from the WebSocket connection, deserialized into the `Out` type.
    ///
    /// If the connection is still opening, this will wait until the connection is established.
    /// If the connection fails to open or is killed while waiting, an error will be returned.
    ///
    /// This method returns an error if the connection is closed since we assume closed connections
    /// are a "failure".
    pub async fn recv(&mut self) -> Result<Out, WebsocketError>
    where
        Out: DeserializeOwned,
        E: Encoding,
    {
        self.connect().await;

        let result = self
            .connection
            .as_ref()
            .as_deref()
            .ok_or_else(WebsocketError::closed_away)?
            .as_ref()
            .map_err(|_| WebsocketError::AlreadyClosed)?
            .recv()
            .await;

        if let Err(WebsocketError::ConnectionClosed { .. }) = result.as_ref() {
            self.received_shutdown();
        }

        result
    }

    /// Set the WebSocket connection.
    ///
    /// This method takes a `Result<Websocket<In, Out, E>, Err>`, allowing you to drive the connection
    /// into an errored state manually.
    pub fn set<Err: Into<CapturedError>>(&mut self, socket: Result<Websocket<In, Out, E>, Err>) {
        match socket {
            Ok(_) => self.status.set(WebsocketState::Open),
            Err(_) => self.status.set(WebsocketState::FailedToConnect),
        }

        self.connection.set(Some(socket.map_err(|e| e.into())));
        self.waker.wake(());
    }

    /// Mark the WebSocket as closed. This is called internally when the connection is closed.
    fn received_shutdown(&self) {
        let mut _self = *self;
        _self.status.set(WebsocketState::Closed);
        _self.waker.wake(());
    }
}

impl<In, Out, E> Copy for UseWebsocket<In, Out, E> {}
impl<In, Out, E> Clone for UseWebsocket<In, Out, E> {
    fn clone(&self) -> Self {
        *self
    }
}

#[derive(Debug, Clone, PartialEq, Copy)]
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

/// A WebSocket connection that can send and receive messages of type `In` and `Out`.
pub struct Websocket<In = String, Out = String, E = JsonEncoding> {
    protocol: Option<String>,

    #[allow(clippy::type_complexity)]
    _in: std::marker::PhantomData<fn() -> (In, Out, E)>,

    #[cfg(not(target_arch = "wasm32"))]
    native: Option<native::SplitSocket>,

    #[cfg(feature = "web")]
    web: Option<WebsysSocket>,

    response: Option<axum::response::Response>,
}

impl<I, O, E> Websocket<I, O, E> {
    pub async fn recv(&self) -> Result<O, WebsocketError>
    where
        O: DeserializeOwned,
        E: Encoding,
    {
        loop {
            let msg = self.recv_raw().await?;
            match msg {
                Message::Text(text) => {
                    let e: O =
                        E::decode(text.into()).ok_or_else(WebsocketError::deserialization)?;
                    return Ok(e);
                }
                Message::Binary(bytes) => {
                    let e: O = E::decode(bytes).ok_or_else(WebsocketError::deserialization)?;
                    return Ok(e);
                }
                Message::Close { code, reason } => {
                    return Err(WebsocketError::ConnectionClosed {
                        code,
                        description: reason,
                    });
                }

                // todo - are we supposed to response to pings?
                Message::Ping(_bytes) => continue,
                Message::Pong(_bytes) => continue,
            }
        }
    }

    /// Send a typed message over the WebSocket connection.
    ///
    /// This method serializes the message using the specified encoding `E` before sending it.
    /// The message will always be sent as a binary message, even if the encoding is valid UTF-8
    /// like JSON.
    pub async fn send(&self, msg: I) -> Result<(), WebsocketError>
    where
        I: Serialize,
        E: Encoding,
    {
        let bytes = E::to_bytes(&msg).ok_or_else(WebsocketError::serialization)?;
        self.send_raw(Message::Binary(bytes)).await
    }

    /// Send a raw message over the WebSocket connection.
    ///
    /// This method allows sending text, binary, ping, pong, and close messages directly.
    pub async fn send_raw(&self, message: Message) -> Result<(), WebsocketError> {
        #[cfg(feature = "web")]
        if cfg!(target_arch = "wasm32") {
            let mut sender = self
                .web
                .as_ref()
                .ok_or_else(|| WebsocketError::Uninitialized)?
                .sender
                .lock()
                .await;

            match message {
                Message::Text(s) => {
                    sender.send(gloo_net::websocket::Message::Text(s)).await?;
                }
                Message::Binary(bytes) => {
                    sender
                        .send(gloo_net::websocket::Message::Bytes(bytes.into()))
                        .await?;
                }
                Message::Close { .. } => {
                    sender.close().await?;
                }
                Message::Ping(_bytes) => return Ok(()),
                Message::Pong(_bytes) => return Ok(()),
            }

            return Ok(());
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut sender = self
                .native
                .as_ref()
                .ok_or_else(|| WebsocketError::Uninitialized)?
                .sender
                .lock()
                .await;

            sender
                .send(message.into())
                .await
                .map_err(WebsocketError::from)?;
        }

        Ok(())
    }

    /// Receive a raw message from the WebSocket connection.
    pub async fn recv_raw(&self) -> Result<Message, WebsocketError> {
        #[cfg(feature = "web")]
        if cfg!(target_arch = "wasm32") {
            let mut conn = self.web.as_ref().unwrap().receiver.lock().await;
            return match conn.next().await {
                Some(Ok(WsMessage::Text(text))) => Ok(Message::Text(text)),
                Some(Ok(WsMessage::Bytes(items))) => Ok(Message::Binary(items.into())),
                Some(Err(e)) => Err(WebsocketError::from(e)),
                None => Err(WebsocketError::closed_away()),
            };
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            use tungstenite::Message as TMessage;
            let mut conn = self.native.as_ref().unwrap().receiver.lock().await;
            return match conn.next().await {
                Some(Ok(res)) => match res {
                    TMessage::Text(utf8_bytes) => Ok(Message::Text(utf8_bytes.to_string())),
                    TMessage::Binary(bytes) => Ok(Message::Binary(bytes)),
                    TMessage::Close(Some(cf)) => Ok(Message::Close {
                        code: cf.code.into(),
                        reason: cf.reason.to_string(),
                    }),
                    TMessage::Close(None) => Ok(Message::Close {
                        code: CloseCode::Away,
                        reason: "Away".to_string(),
                    }),
                    TMessage::Ping(bytes) => Ok(Message::Ping(bytes)),
                    TMessage::Pong(bytes) => Ok(Message::Pong(bytes)),
                    TMessage::Frame(_frame) => Err(WebsocketError::Unexpected),
                },
                Some(Err(e)) => Err(WebsocketError::from(e)),
                None => Err(WebsocketError::closed_away()),
            };
        }

        unimplemented!("Non web wasm32 clients are not supported yet")
    }

    pub fn protocol(&self) -> Option<&str> {
        self.protocol.as_deref()
    }
}

// no two websockets are ever equal
impl<I, O, E> PartialEq for Websocket<I, O, E> {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

// Create a new WebSocket connection that uses the provided function to handle incoming messages
impl<In, Out, E> IntoResponse for Websocket<In, Out, E> {
    fn into_response(self) -> Response {
        let Some(response) = self.response else {
            return HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "WebSocket response not initialized",
            )
            .into_response();
        };

        response.into_response()
    }
}

impl<I, O, E> FromResponse<UpgradingWebsocket> for Websocket<I, O, E> {
    fn from_response(res: UpgradingWebsocket) -> impl Future<Output = Result<Self, ServerFnError>> {
        async move {
            #[cfg(not(target_arch = "wasm32"))]
            let native = res.native;

            #[cfg(feature = "web")]
            let web = res.web.map(|f| {
                let (sender, receiver) = f.split();
                WebsysSocket {
                    sender: Mutex::new(sender),
                    receiver: Mutex::new(receiver),
                }
            });

            Ok(Websocket {
                protocol: res.protocol,
                #[cfg(not(target_arch = "wasm32"))]
                native,
                #[cfg(feature = "web")]
                web,
                response: None,
                _in: PhantomData,
            })
        }
    }
}

pub struct WebSocketOptions {
    protocols: Vec<String>,
    automatic_reconnect: bool,
    #[cfg(feature = "server")]
    upgrade: Option<axum::extract::ws::WebSocketUpgrade>,
    #[cfg(feature = "server")]
    on_failed_upgrade: Option<Box<dyn FnOnce(axum::Error) + Send + 'static>>,
}

impl WebSocketOptions {
    pub fn new() -> Self {
        Self {
            protocols: Vec::new(),
            automatic_reconnect: false,

            #[cfg(feature = "server")]
            upgrade: None,

            #[cfg(feature = "server")]
            on_failed_upgrade: None,
        }
    }

    /// Automatically reconnect if the connection is lost. This uses an exponential backoff strategy.
    pub fn with_automatic_reconnect(mut self) -> Self {
        self.automatic_reconnect = true;
        self
    }

    #[cfg(feature = "server")]
    pub fn on_failed_upgrade(
        mut self,
        callback: impl FnOnce(axum::Error) + Send + 'static,
    ) -> Self {
        self.on_failed_upgrade = Some(Box::new(callback));

        self
    }

    #[cfg(feature = "server")]
    pub fn on_upgrade<F, Fut, In, Out, Enc>(mut self, callback: F) -> Websocket<In, Out, Enc>
    where
        F: FnOnce(TypedWebsocket<In, Out, Enc>) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        let on_failed_upgrade = self.on_failed_upgrade.take();
        let response = self
            .upgrade
            .unwrap()
            .on_failed_upgrade(|e| {
                if let Some(callback) = on_failed_upgrade {
                    callback(e);
                }
            })
            .on_upgrade(|socket| {
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
            // the protocol is none here since it won't be accessible until after the upgrade
            protocol: None,
            response: Some(response),
            _in: PhantomData,

            #[cfg(not(target_arch = "wasm32"))]
            native: None,

            #[cfg(feature = "web")]
            web: None,
        }
    }
}

impl Default for WebSocketOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoRequest<UpgradingWebsocket> for WebSocketOptions {
    fn into_request(
        self,
        request: ClientRequest,
    ) -> impl Future<Output = std::result::Result<UpgradingWebsocket, RequestError>> + 'static {
        async move {
            #[cfg(feature = "web")]
            if cfg!(target_arch = "wasm32") {
                let url_path = request.url().path();
                let url_query = request.url().query();
                let url_fragment = request.url().fragment();
                let path_and_query = format!(
                    "{}{}{}",
                    url_path,
                    url_query.map_or("".to_string(), |q| format!("?{q}")),
                    url_fragment.map_or("".to_string(), |f| format!("#{f}"))
                );

                let socket = gloo_net::websocket::futures::WebSocket::open_with_protocols(
                    // ! very important we use the path here and not the full url on web.
                    // for as long as serverfns are meant to target the same origin, this is fine.
                    &path_and_query,
                    &self
                        .protocols
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<_>>(),
                )
                .unwrap();

                return Ok(UpgradingWebsocket {
                    protocol: Some(socket.protocol()),
                    web: Some(socket),
                    #[cfg(not(target_arch = "wasm32"))]
                    native: None,
                });
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                let response = native::send_request(request, &self.protocols)
                    .await
                    .unwrap();

                let (inner, protocol) = response
                    .into_stream_and_protocol(self.protocols, None)
                    .await
                    .unwrap();

                return Ok(UpgradingWebsocket {
                    protocol,
                    native: Some(inner),
                    #[cfg(feature = "web")]
                    web: None,
                });
            }

            unimplemented!("Non web wasm32 clients are not supported yet")
        }
    }
}

impl<S: Send> FromRequest<S> for WebSocketOptions {
    type Rejection = axum::response::Response;

    fn from_request(
        _req: Request,
        _: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        #[cfg(not(feature = "server"))]
        return async move { Err(StatusCode::NOT_IMPLEMENTED.into_response()) };

        #[cfg(feature = "server")]
        async move {
            let ws = match axum::extract::ws::WebSocketUpgrade::from_request(_req, &()).await {
                Ok(ws) => ws,
                Err(rejection) => return Err(rejection.into_response()),
            };

            Ok(WebSocketOptions {
                protocols: vec![],
                automatic_reconnect: false,
                upgrade: Some(ws),
                on_failed_upgrade: None,
            })
        }
    }
}

#[doc(hidden)]
pub struct UpgradingWebsocket {
    protocol: Option<String>,

    #[cfg(feature = "web")]
    web: Option<gloo_net::websocket::futures::WebSocket>,

    #[cfg(not(target_arch = "wasm32"))]
    native: Option<native::SplitSocket>,
}

unsafe impl Send for UpgradingWebsocket {}
unsafe impl Sync for UpgradingWebsocket {}

#[cfg(feature = "server")]
pub struct TypedWebsocket<In, Out, E = JsonEncoding> {
    _in: std::marker::PhantomData<fn() -> In>,
    _out: std::marker::PhantomData<fn() -> Out>,
    _enc: std::marker::PhantomData<fn() -> E>,

    inner: axum::extract::ws::WebSocket,
}

#[cfg(feature = "server")]
impl<In: DeserializeOwned, Out: Serialize, E: Encoding> TypedWebsocket<In, Out, E> {
    /// Receive an incoming message from the client.
    ///
    /// Returns `None` if the stream has closed.
    pub async fn recv(&mut self) -> Result<In, WebsocketError> {
        use axum::extract::ws::Message as AxumMessage;

        loop {
            let Some(res) = self.inner.next().await else {
                return Err(WebsocketError::closed_away());
            };

            match res {
                Ok(res) => match res {
                    AxumMessage::Text(utf8_bytes) => {
                        let e: In = E::decode(utf8_bytes.into())
                            .ok_or_else(WebsocketError::deserialization)?;
                        return Ok(e);
                    }
                    AxumMessage::Binary(bytes) => {
                        let e: In = E::decode(bytes).ok_or_else(WebsocketError::deserialization)?;
                        return Ok(e);
                    }

                    AxumMessage::Close(Some(close_frame)) => {
                        return Err(WebsocketError::ConnectionClosed {
                            code: close_frame.code.into(),
                            description: close_frame.reason.to_string(),
                        });
                    }
                    AxumMessage::Close(None) => return Err(WebsocketError::AlreadyClosed),

                    AxumMessage::Ping(_bytes) => continue,
                    AxumMessage::Pong(_bytes) => continue,
                },
                Err(_res) => return Err(WebsocketError::closed_away()),
            }
        }
    }

    /// Send an outgoing message.
    pub async fn send(&mut self, msg: Out) -> Result<(), WebsocketError> {
        use axum::extract::ws::Message;

        let to_bytes = E::to_bytes(&msg).ok_or_else(|| {
            WebsocketError::Serialization(anyhow::anyhow!("Failed to serialize message").into())
        })?;

        self.inner
            .send(Message::Binary(to_bytes))
            .await
            .map_err(|_err| WebsocketError::AlreadyClosed)
    }

    /// Receive another message.
    ///
    /// Returns `None` if the stream has closed.
    pub async fn recv_raw(&mut self) -> Result<Message, WebsocketError> {
        use axum::extract::ws::Message as AxumMessage;

        let message = self
            .inner
            .next()
            .await
            .ok_or_else(WebsocketError::closed_away)?
            .map_err(|_| WebsocketError::AlreadyClosed)?;

        Ok(match message {
            AxumMessage::Text(utf8_bytes) => Message::Text(utf8_bytes.to_string()),
            AxumMessage::Binary(bytes) => Message::Binary(bytes),
            AxumMessage::Ping(bytes) => Message::Ping(bytes),
            AxumMessage::Pong(bytes) => Message::Pong(bytes),
            AxumMessage::Close(close_frame) => Message::Close {
                code: close_frame
                    .clone()
                    .map_or(CloseCode::Away, |cf| cf.code.into()),
                reason: close_frame.map_or("Away".to_string(), |cf| cf.reason.to_string()),
            },
        })
    }

    /// Send a message.
    pub async fn send_raw(&mut self, msg: Message) -> Result<(), WebsocketError> {
        let real = match msg {
            Message::Text(text) => axum::extract::ws::Message::Text(text.into()),
            Message::Binary(bytes) => axum::extract::ws::Message::Binary(bytes),
            Message::Ping(bytes) => axum::extract::ws::Message::Ping(bytes),
            Message::Pong(bytes) => axum::extract::ws::Message::Pong(bytes),
            Message::Close { code, reason } => {
                axum::extract::ws::Message::Close(Some(axum::extract::ws::CloseFrame {
                    code: code.into(),
                    reason: reason.into(),
                }))
            }
        };

        self.inner
            .send(real)
            .await
            .map_err(|_err| WebsocketError::AlreadyClosed)
    }

    /// Return the selected WebSocket subprotocol, if one has been chosen.
    pub fn protocol(&self) -> Option<&http::HeaderValue> {
        self.inner.protocol()
    }

    /// Get a mutable reference to the underlying Axum WebSocket.
    pub fn socket(&mut self) -> &mut axum::extract::ws::WebSocket {
        &mut self.inner
    }
}

#[derive(thiserror::Error, Debug)]
pub enum WebsocketError {
    #[error("Connection closed")]
    ConnectionClosed {
        code: CloseCode,
        description: String,
    },

    #[error("WebSocket already closed")]
    AlreadyClosed,

    #[error("WebSocket capacity reached")]
    Capacity,

    #[error("An unexpected internal error occurred")]
    Unexpected,

    #[error("WebSocket is not initialized on this platform")]
    Uninitialized,

    #[cfg(not(target_arch = "wasm32"))]
    #[error("websocket upgrade failed")]
    Handshake(#[from] native::HandshakeError),

    #[error("reqwest error")]
    Reqwest(#[from] reqwest::Error),

    #[cfg(not(target_arch = "wasm32"))]
    #[error("tungstenite error")]
    Tungstenite(#[from] tungstenite::Error),

    /// Error during serialization/deserialization.
    #[error("error during serialization/deserialization")]
    Deserialization(Box<dyn std::error::Error + Send + Sync>),

    /// Error during serialization/deserialization.
    #[error("error during serialization/deserialization")]
    Serialization(Box<dyn std::error::Error + Send + Sync>),

    /// Error during serialization/deserialization.
    #[error("serde_json error")]
    Json(#[from] serde_json::Error),

    /// Error during serialization/deserialization.
    #[error("ciborium error")]
    Cbor(#[from] ciborium::de::Error<std::io::Error>),
}

#[cfg(feature = "web")]
impl From<gloo_net::websocket::WebSocketError> for WebsocketError {
    fn from(value: gloo_net::websocket::WebSocketError) -> Self {
        use gloo_net::websocket::WebSocketError;
        match value {
            WebSocketError::ConnectionError => WebsocketError::AlreadyClosed,
            WebSocketError::ConnectionClose(close_event) => WebsocketError::ConnectionClosed {
                code: close_event.code.into(),
                description: close_event.reason,
            },
            WebSocketError::MessageSendError(_js_error) => WebsocketError::Unexpected,
            _ => WebsocketError::Unexpected,
        }
    }
}

impl WebsocketError {
    pub fn closed_away() -> Self {
        Self::ConnectionClosed {
            code: CloseCode::Normal,
            description: "Connection closed normally".into(),
        }
    }

    pub fn deserialization() -> Self {
        Self::Deserialization(anyhow::anyhow!("Failed to deserialize message").into())
    }

    pub fn serialization() -> Self {
        Self::Serialization(anyhow::anyhow!("Failed to serialize message").into())
    }
}

#[cfg(feature = "web")]
struct WebsysSocket {
    sender: Mutex<SplitSink<WsWebsocket, WsMessage>>,
    receiver: Mutex<SplitStream<WsWebsocket>>,
}

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
    Ping(Bytes),

    /// A pong message with the specified payload.
    ///
    /// The payload here must have a length less than 125 bytes.
    ///
    /// # WASM
    ///
    /// This variant is ignored for WASM targets.
    Pong(Bytes),

    /// A close message.
    ///
    /// Sending this will not close the connection, though the remote peer will likely close the connection after receiving this.
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

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::borrow::Cow;

    use crate::ClientRequest;

    use super::{CloseCode, Message, WebsocketError};
    use reqwest::{
        header::{HeaderName, HeaderValue},
        Response, StatusCode, Version,
    };
    use tungstenite::protocol::WebSocketConfig;

    pub(crate) struct SplitSocket {
        pub sender: futures_util::lock::Mutex<
            async_tungstenite::WebSocketSender<tokio_util::compat::Compat<reqwest::Upgraded>>,
        >,

        pub receiver: futures_util::lock::Mutex<
            async_tungstenite::WebSocketReceiver<tokio_util::compat::Compat<reqwest::Upgraded>>,
        >,
    }

    pub async fn send_request(
        request: ClientRequest,
        protocols: &[String],
    ) -> Result<WebSocketResponse, WebsocketError> {
        let request_builder = request.new_reqwest_request();
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
        ) -> Result<(SplitSocket, Option<String>), WebsocketError> {
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

            let split: (
                async_tungstenite::WebSocketSender<tokio_util::compat::Compat<reqwest::Upgraded>>,
                async_tungstenite::WebSocketReceiver<tokio_util::compat::Compat<reqwest::Upgraded>>,
            ) = inner.split();

            let split_socket = SplitSocket {
                sender: futures_util::lock::Mutex::new(split.0),
                receiver: futures_util::lock::Mutex::new(split.1),
            };

            Ok((split_socket, protocol))
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
