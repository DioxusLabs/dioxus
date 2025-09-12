use dioxus_core::{RenderError, Result};
use dioxus_hooks::Resource;
use dioxus_signals::{Loader, Signal};
use serde::Serialize;
use std::{marker::PhantomData, prelude::rust_2024::Future};
use tokio_tungstenite::tungstenite::Error as WsError;

use crate::Websocket;

pub fn use_loader<
    F: Future<Output = anyhow::Result<T, E>>,
    T: 'static + PartialEq,
    E: Into<anyhow::Error>,
>(
    // pub fn use_loader<F: Future<Output = Result<T, E>>, T: 'static, E: Into<anyhow::Error>>(
    f: impl FnMut() -> F,
) -> Result<Loader<T>, RenderError> {
    todo!()
}

pub fn use_action<F: Future<Output = Result<O, E>>, E, I, O>(
    f: impl FnOnce(I) -> F,
) -> Action<I, O> {
    todo!()
}

pub struct Action<I, T> {
    _t: PhantomData<*const T>,
    _i: PhantomData<*const I>,
}
impl<I, T> Action<I, T> {
    pub async fn dispatch(&mut self, input: I) -> Result<T> {
        todo!()
    }
}
impl<I, T> std::ops::Deref for Action<I, T> {
    type Target = fn(I);

    fn deref(&self) -> &Self::Target {
        todo!()
    }
}

impl<I, T> Clone for Action<I, T> {
    fn clone(&self) -> Self {
        todo!()
    }
}
impl<I, T> Copy for Action<I, T> {}

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

    pub async fn send(&mut self, msg: impl Serialize) -> Result<(), WsError> {
        todo!()
    }
}

pub fn with_axum_router<E, F: Future<Output = Result<axum::routing::Router<()>, E>>>(
    f: impl FnMut() -> F,
) {
}
