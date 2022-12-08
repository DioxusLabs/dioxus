use std::sync::Arc;

use async_rwlock::RwLock;
use dioxus::{core::Component, prelude::ScopeId};
use dioxus_router_core::{RouterMessage, RouterState};
use futures_channel::mpsc::UnboundedSender;

#[derive(Clone)]
pub(crate) struct RouterContext {
    pub(crate) state: Arc<RwLock<RouterState<Component>>>,
    pub(crate) sender: UnboundedSender<RouterMessage<ScopeId>>,
}
