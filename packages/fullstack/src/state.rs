//! A shared pool of renderers for efficient server side rendering.
use crate::{document::ServerDocument, ssr::SsrRendererPool, ProvideServerContext, ServeConfig};
use crate::{
    streaming::{Mount, StreamingRenderer},
    DioxusServerContext,
};
use dioxus_cli_config::base_path;
use dioxus_core::{
    has_context, provide_error_boundary, DynamicNode, ErrorContext, ScopeId, SuspenseContext,
    VNode, VirtualDom,
};
use dioxus_fullstack_hooks::history::provide_fullstack_history_context;
use dioxus_fullstack_hooks::{HydrationContext, SerializedHydrationData};
use dioxus_fullstack_hooks::{StreamingContext, StreamingStatus};
use dioxus_isrg::{CachedRender, IncrementalRendererError, RenderFreshness};
use dioxus_router::ParseRouteError;
use dioxus_ssr::Renderer;
use futures_channel::mpsc::Sender;
use futures_util::{Stream, StreamExt};
use std::{
    collections::HashMap,
    fmt::Write,
    future::Future,
    marker::PhantomData,
    rc::Rc,
    sync::{Arc, RwLock},
};
use tokio::task::JoinHandle;

use crate::StreamingMode;

pub struct ServerState<T> {
    _t: PhantomData<*const T>,
}

impl<T> ServerState<T> {
    fn get(&self) -> &T {
        todo!()
    }

    pub const fn new(f: fn() -> T) -> Self {
        Self { _t: PhantomData }
    }
}

impl<T> std::ops::Deref for ServerState<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        todo!()
    }
}
impl<T> std::ops::DerefMut for ServerState<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        todo!()
    }
}

unsafe impl<T> Send for ServerState<T> {}
unsafe impl<T> Sync for ServerState<T> {}
