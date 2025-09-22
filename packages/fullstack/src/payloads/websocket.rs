use axum::extract::{FromRequest, Request};
use axum_core::response::{IntoResponse, Response};
use bytes::Bytes;

use crate::{FromResponse, IntoRequest, ServerFnError};

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
    response: Option<axum::response::Response>,
}

impl<I, O> PartialEq for Websocket<I, O> {
    fn eq(&self, other: &Self) -> bool {
        todo!()
    }
}

impl<I, O> FromResponse for Websocket<I, O> {
    fn from_response(
        res: reqwest::Response,
    ) -> impl Future<Output = Result<Self, ServerFnError>> + Send {
        async move { todo!() }
    }
}

pub struct WebSocketOptions {
    _private: (),
    #[cfg(feature = "server")]
    upgrade: Option<axum::extract::ws::WebSocketUpgrade>,
}

impl WebSocketOptions {
    pub fn new() -> Self {
        Self {
            _private: (),

            #[cfg(feature = "server")]
            upgrade: None,
        }
    }

    #[cfg(feature = "server")]
    pub fn on_upgrade<F, Fut>(self, f: F) -> Websocket
    where
        F: FnOnce(axum::extract::ws::WebSocket) -> Fut + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        let response = self.upgrade.unwrap().on_upgrade(|socket| async move {
            //
        });

        Websocket {
            response: Some(response),
            _in: PhantomData,
            _out: PhantomData,
        }
    }
}

impl IntoRequest for WebSocketOptions {
    fn into_request(
        input: Self,
        builder: reqwest::RequestBuilder,
    ) -> impl Future<Output = std::result::Result<reqwest::Response, reqwest::Error>> + Send + 'static
    {
        async move { todo!() }
    }
}

#[cfg(feature = "server")]
impl<S: Send> FromRequest<S> for WebSocketOptions {
    #[doc = " If the extractor fails it\'ll use this \"rejection\" type. A rejection is"]
    #[doc = " a kind of error that can be converted into a response."]
    type Rejection = axum::http::StatusCode;

    #[doc = " Perform the extraction."]
    fn from_request(
        req: Request,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let ws = match axum::extract::ws::WebSocketUpgrade::from_request(req, &()).await {
                Ok(ws) => ws,
                Err(rejection) => todo!(),
            };

            Ok(WebSocketOptions {
                _private: (),
                upgrade: Some(ws),
            })
        }
    }
}
