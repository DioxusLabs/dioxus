use dioxus_core::prelude::Callback;
use rustc_hash::FxHashMap;
use std::{cell::RefCell, rc::Rc};
use wry::{http::Request, RequestAsyncResponder};

/// A request for an asset within dioxus-desktop.
pub type AssetRequest = Request<Vec<u8>>;

pub struct AssetHandler {
    f: Callback<(AssetRequest, RequestAsyncResponder)>,
}

#[derive(Clone)]
pub struct AssetHandlers {
    handlers: Rc<RefCell<FxHashMap<String, AssetHandler>>>,
}

impl AssetHandlers {
    pub fn new() -> Self {
        AssetHandlers {
            handlers: Default::default(),
        }
    }

    pub fn has_handler(&self, name: &str) -> bool {
        self.handlers.borrow().contains_key(name)
    }

    pub fn handle_request(
        &self,
        name: &str,
        request: AssetRequest,
        responder: RequestAsyncResponder,
    ) {
        if let Some(handler) = self.handlers.borrow().get(name) {
            handler.f.call((request, responder));
        }
    }

    pub fn register_handler(
        &self,
        name: String,
        f: Callback<(AssetRequest, RequestAsyncResponder)>,
    ) {
        self.handlers.borrow_mut().insert(name, AssetHandler { f });
    }

    pub fn remove_handler(&self, name: &str) -> Option<AssetHandler> {
        self.handlers.borrow_mut().remove(name)
    }
}
