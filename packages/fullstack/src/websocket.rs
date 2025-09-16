use axum_core::response::{IntoResponse, Response};
use bytes::Bytes;

use crate::ServerFnError;

use dioxus_core::{RenderError, Result};
use dioxus_hooks::Loader;
use dioxus_hooks::Resource;
use dioxus_signals::Signal;
use serde::{de::DeserializeOwned, Serialize};
use std::{marker::PhantomData, prelude::rust_2024::Future};

pub fn use_websocket<E, F: Future<Output = Result<Websocket, E>>>(
    f: impl FnOnce() -> F,
) -> WebsocketHandle {
    todo!()
}
pub struct WebsocketHandle {}
impl Clone for WebsocketHandle {
    fn clone(&self) -> Self {
        todo!()
    }
}
impl Copy for WebsocketHandle {}

impl WebsocketHandle {
    pub fn connecting(&self) -> bool {
        todo!()
    }

    #[cfg(feature = "server")]
    pub async fn send(
        &mut self,
        msg: impl Serialize,
    ) -> Result<(), tokio_tungstenite::tungstenite::Error> {
        todo!()
    }
}

impl<In: Serialize, Out: DeserializeOwned> Websocket<In, Out> {
    #[cfg(feature = "server")]
    pub fn raw<O, F: Future<Output = O>>(
        f: impl FnOnce(
            axum::extract::ws::WebSocket, // tokio_tungstenite::tungstenite::protocol::WebSocket<tokio::net::TcpStream>,
                                          // tokio_tungstenite::tungstenite::stream::Stream<
                                          //         tokio::net::TcpStream,
                                          //         tokio_native_tls::TlsStream<tokio::net::TcpStream>,
                                          //     >,
        ) -> F,
    ) -> Self {
        todo!()
    }

    pub async fn send(&self, msg: In) -> Result<(), ServerFnError> {
        todo!()
    }

    pub async fn recv(&mut self) -> Result<Out, ServerFnError> {
        todo!()
    }
}

// Create a new WebSocket connection that uses the provided function to handle incoming messages
impl<In, Out> IntoResponse for Websocket<In, Out> {
    fn into_response(self) -> Response {
        todo!()
    }
}

pub struct TypedWebsocket<In, Out> {
    _in: std::marker::PhantomData<In>,
    _out: std::marker::PhantomData<Out>,
}

/// A WebSocket connection that can send and receive messages of type `In` and `Out`.
pub struct Websocket<In = String, Out = String> {
    _in: std::marker::PhantomData<In>,
    _out: std::marker::PhantomData<Out>,
}
