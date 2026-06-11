use rustc_hash::FxHashMap;
use std::{cell::RefCell, rc::Rc};
use wry::{RequestAsyncResponder, http::Request};

/// A request for an asset within dioxus-desktop.
pub type AssetRequest = Request<Vec<u8>>;

pub struct AssetHandler {
    f: Box<dyn FnMut(AssetRequest, RequestAsyncResponder)>,
}

#[derive(Clone)]
pub struct AssetHandlerRegistry {
    handlers: Rc<RefCell<FxHashMap<String, AssetHandler>>>,
}

impl AssetHandlerRegistry {
    pub fn new() -> Self {
        AssetHandlerRegistry {
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
        let Some(mut handler) = self.handlers.borrow_mut().remove(name) else {
            return;
        };

        // Avoid handler being already borrowed on android
        #[cfg(target_os = "android")]
        let _lock = crate::android_sync_lock::android_runtime_lock();

        (handler.f)(request, responder);

        if !self.handlers.borrow().contains_key(name) {
            self.handlers.borrow_mut().insert(name.to_string(), handler);
        }
    }

    pub fn register_handler(
        &self,
        name: String,
        f: impl FnMut(AssetRequest, RequestAsyncResponder) + 'static,
    ) {
        self.handlers
            .borrow_mut()
            .insert(name, AssetHandler { f: Box::new(f) });
    }

    pub fn remove_handler(&self, name: &str) -> Option<AssetHandler> {
        self.handlers.borrow_mut().remove(name)
    }
}

#[cfg(test)]
mod tests {
    use super::AssetHandlerRegistry;

    #[test]
    fn register_handler_does_not_require_dioxus_runtime() {
        let registry = AssetHandlerRegistry::new();

        registry.register_handler("custom".to_string(), |_request, _responder| {});

        assert!(registry.has_handler("custom"));
    }
}
