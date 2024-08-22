use std::rc::Rc;

use dioxus_core::ScopeId;
use dioxus_document::{Document, Eval};

/// Provides the WebEvalProvider through [`ScopeId::provide_context`].
pub fn init_document() {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let provider: Rc<dyn Document> = Rc::new(WebDocument { document });

    if ScopeId::ROOT.has_context::<Rc<dyn Document>>().is_none() {
        ScopeId::ROOT.provide_context(provider);
    }
}

/// The web-target's document provider.
pub struct WebDocument {
    document: web_sys::Document,
}

impl Document for WebDocument {
    fn eval(&self, js: String) -> Eval {
        let (tx, eval) = Eval::from_parts();

        // todo: this deserialize is probably wrong.
        _ = match js_sys::eval(&js) {
            Ok(ok) => tx.send(Ok(serde_wasm_bindgen::from_value(ok).unwrap())),
            Err(_err) => tx.send(Err(dioxus_document::EvalError::Communication(
                "eval failed".to_string(),
            ))),
        };

        eval
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
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
}
