use dioxus_core::{RenderError, Result};
use dioxus_hooks::Loader;
use dioxus_hooks::Resource;
use dioxus_signals::Signal;
use serde::Serialize;
use std::{marker::PhantomData, prelude::rust_2024::Future};
use tokio_tungstenite::tungstenite::Error as WsError;

use crate::Websocket;

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

pub fn with_router<E, F: Future<Output = Result<axum::routing::Router<()>, E>>>(
    f: impl FnMut() -> F,
) {
}
