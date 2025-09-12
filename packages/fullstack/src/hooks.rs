use dioxus_core::{RenderError, Result};
use dioxus_hooks::Resource;
use dioxus_signals::{Loader, Signal};
use serde::Serialize;
use std::{marker::PhantomData, prelude::rust_2024::Future};
use tokio_tungstenite::tungstenite::Error as WsError;

use crate::Websocket;

/// A hook to create a resource that loads data asynchronously.
///
/// To bubble errors and pending, simply use `?` on the result of the resource read.
///
/// To inspect the state of the resource, you can use the RenderError enum along with the RenderResultExt trait.
pub fn use_loader<
    F: Future<Output = anyhow::Result<T, E>>,
    T: 'static + PartialEq,
    E: Into<anyhow::Error>,
>(
    // pub fn use_loader<F: Future<Output = Result<T, E>>, T: 'static, E: Into<anyhow::Error>>(
    f: impl FnMut() -> F,
) -> Result<Loader<T>, Loading> {
    todo!()
}

#[derive(PartialEq)]
pub enum Loading {
    Pending(LoaderHandle<()>),

    Failed(LoaderHandle<RenderError>),
}

impl std::fmt::Debug for Loading {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Loading::Pending(_) => write!(f, "Loading::Pending"),
            Loading::Failed(_) => write!(f, "Loading::Failed"),
        }
    }
}

impl std::fmt::Display for Loading {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Loading::Pending(_) => write!(f, "Loading is still pending"),
            Loading::Failed(_) => write!(f, "Loading has failed"),
        }
    }
}

impl From<Loading> for RenderError {
    fn from(val: Loading) -> Self {
        todo!()
    }
}

#[derive(PartialEq)]
pub struct LoaderHandle<T> {
    _t: PhantomData<*const T>,
}
impl<T> LoaderHandle<T> {
    pub fn restart(&self) {
        todo!()
    }
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

    pub fn value(&self) -> Option<Signal<T>> {
        todo!()
    }

    pub fn is_loading(&self) -> bool {
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
