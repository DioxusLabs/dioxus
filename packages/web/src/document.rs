use std::rc::Rc;

use dioxus_core::ScopeId;
use dioxus_html::document::{Document, EvalError, Evaluator};
use generational_box::{AnyStorage, GenerationalBox, UnsyncStorage};
use js_sys::Function;
use serde::Serialize;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::{rc::Rc, str::FromStr};
use wasm_bindgen::prelude::*;

#[wasm_bindgen::prelude::wasm_bindgen]
pub struct JSOwner {
    _owner: Box<dyn std::any::Any>,
}

impl JSOwner {
    pub fn new(owner: impl std::any::Any) -> Self {
        Self {
            _owner: Box::new(owner),
        }
    }
}

#[wasm_bindgen::prelude::wasm_bindgen(module = "/src/js/eval.js")]
extern "C" {
    pub type WebDioxusChannel;

    #[wasm_bindgen(constructor)]
    pub fn new(owner: JSOwner) -> WebDioxusChannel;

    #[wasm_bindgen(method, js_name = "rustSend")]
    pub fn rust_send(this: &WebDioxusChannel, value: wasm_bindgen::JsValue);

    #[wasm_bindgen(method, js_name = "rustRecv")]
    pub async fn rust_recv(this: &WebDioxusChannel) -> wasm_bindgen::JsValue;

    #[wasm_bindgen(method)]
    pub fn send(this: &WebDioxusChannel, value: wasm_bindgen::JsValue);

    #[wasm_bindgen(method)]
    pub async fn recv(this: &WebDioxusChannel) -> wasm_bindgen::JsValue;

    #[wasm_bindgen(method)]
    pub fn weak(this: &WebDioxusChannel) -> WeakDioxusChannel;

    pub type WeakDioxusChannel;

    #[wasm_bindgen(method, js_name = "rustSend")]
    pub fn rust_send(this: &WeakDioxusChannel, value: wasm_bindgen::JsValue);

    #[wasm_bindgen(method, js_name = "rustRecv")]
    pub async fn rust_recv(this: &WeakDioxusChannel) -> wasm_bindgen::JsValue;
}

/// Provides the WebEvalProvider through [`ScopeId::provide_context`].
pub fn init_document() {
    let provider = WebDocument::get();
    if ScopeId::ROOT.has_context::<Rc<dyn Document>>().is_none() {
        ScopeId::ROOT.provide_context(provider);
    }
}

/// The web-target's document provider.
pub struct WebDocument {
    document: web_sys::Document,
}

impl WebDocument {
    /// Get the web document provider
    pub fn get() -> Rc<dyn Document> {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let provider: Rc<dyn Document> = Rc::new(WebDocument { document });
        provider
    }
}

impl Document for WebDocument {
    fn eval(&self, js: String) -> Eval {
        let (tx, eval) = Eval::from_parts();

        // todo: this deserialize is probably wrong.
        _ = match js_sys::eval(&js) {
            Ok(ok) => {
                tracing::trace!("eval result: {ok:#?}");
                let msg = serde_wasm_bindgen::from_value(ok).unwrap_or_default();

                tx.send(Ok(msg))
            }
            Err(_err) => tx.send(Err(dioxus_document::EvalError::Communication(
                "eval failed".to_string(),
            ))),
        };

        eval
    }

    fn set_title(&self, title: String) {
        self.document.set_title(&title);
    }

    fn create_head_element(
        &self,
        name: &str,
        attributes: Vec<(&str, String)>,
        contents: Option<String>,
    ) {
        if let Some(head) = self.document.head() {
            let element = self.document.create_element(name).unwrap();
            for (name, value) in attributes {
                element.set_attribute(name, &value).unwrap();
            }
            if let Some(contents) = contents {
                element.set_inner_html(&contents);
            }
            head.append_child(&element.into()).unwrap();
        }
    }

    fn current_route(&self) -> String {
        todo!()
    }

    fn go_back(&self) {
        todo!()
    }

    fn go_forward(&self) {
        todo!()
    }

    fn push_route(&self, route: String) {
        todo!()
    }

    fn replace_route(&self, path: String) {
        todo!()
    }
}
