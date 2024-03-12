use dioxus_core::prelude::{Runtime, RuntimeGuard, ScopeId};
use rustc_hash::FxHashMap;
use std::{cell::RefCell, rc::Rc};
use wry::{http::Request, RequestAsyncResponder};

///
pub type AssetRequest = Request<Vec<u8>>;

pub struct AssetHandler {
    f: Box<dyn Fn(AssetRequest, RequestAsyncResponder) + 'static>,
    scope: ScopeId,
}

#[derive(Clone)]
pub struct AssetHandlerRegistry {
    dom_rt: Rc<Runtime>,
    handlers: Rc<RefCell<FxHashMap<String, AssetHandler>>>,
}

impl AssetHandlerRegistry {
    pub fn new(dom_rt: Rc<Runtime>) -> Self {
        AssetHandlerRegistry {
            dom_rt,
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
            // Push the runtime onto the stack
            let _guard = RuntimeGuard::new(self.dom_rt.clone());

            // And run the handler in the scope of the component that created it
            self.dom_rt
                .on_scope(handler.scope, || (handler.f)(request, responder));
        }
    }

    pub fn register_handler(
        &self,
        name: String,
        f: Box<dyn Fn(AssetRequest, RequestAsyncResponder) + 'static>,
        scope: ScopeId,
    ) {
        self.handlers
            .borrow_mut()
            .insert(name, AssetHandler { f, scope });
    }

    pub fn remove_handler(&self, name: &str) -> Option<AssetHandler> {
        self.handlers.borrow_mut().remove(name)
    }
}
