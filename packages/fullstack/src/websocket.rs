use axum_core::response::{IntoResponse, Response};
use bytes::Bytes;

use crate::{FromResponse, ServerFnError};

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

impl<I, O> FromResponse for Websocket<I, O> {
    fn from_response(
        res: reqwest::Response,
    ) -> impl Future<Output = Result<Self, ServerFnError>> + Send {
        async move { todo!() }
    }
}

trait GoodServerFn<M> {}

trait IntoRequest {}

struct M1;
impl<O, E, F> GoodServerFn<M1> for F where F: FnMut() -> Result<O, E> {}

struct M2;
impl<O, E, F, A> GoodServerFn<(M2, (O, E, A))> for F
where
    F: FnMut(A) -> Result<O, E>,
    A: IntoRequest,
{
}

struct M3;
impl<O, E, F, A> GoodServerFn<(M3, (O, E, A))> for F
where
    F: FnMut(A) -> Result<O, E>,
    A: DeserializeOwned,
{
}

struct M4;
impl<O, E, F, A, B> GoodServerFn<(M4, (O, E, A, B))> for F
where
    F: FnMut(A, B) -> Result<O, E>,
    A: DeserializeOwned,
    B: DeserializeOwned,
{
}

struct M5;
impl<O, E, F, A, B, C> GoodServerFn<(M5, (O, E, A, B, C))> for F
where
    F: FnMut(A, B, C) -> Result<O, E>,
    A: DeserializeOwned,
    B: DeserializeOwned,
    C: DeserializeOwned,
{
}
/*
async fn do_thing(a: i32, b: String) -> O {
}

client steps:
- ** encode arguments ( Struct { queries, url, method, body })
- send to server, await response
- ** decode response ( FromResponse )

server steps:
- ** decode args from request
    - call function
- ** encode response


client is optional...



a -> Result<T, E>

on client
-> send.await -> Result<T, E> (reqwest err -> serverfn err -> user err)
-> .await -> Result<T, E> (reqwest ok -> user err)

our "encoding" can just be us implicitly wrapping the types with Json<T> or Form<T> as needed
*/

mod new_impl {
    use axum::{Form, Json};

    macro_rules! do_it {
        () => {};
    }

    trait RequestEncoder<O> {
        type Output;
    }

    trait Encoding<T> {
        const CONTENT_TYPE: &'static str;
    }

    fn fetch() {
        //
    }
}
