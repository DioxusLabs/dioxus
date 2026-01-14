use dioxus_core::queue_effect;
use dioxus_core::ScopeId;
use dioxus_core::{provide_context, Runtime};
use dioxus_document::{Document, Eval, LinkProps, MetaProps, ScriptProps, StyleProps};
use dioxus_history::History;
use dioxus_web_eval::WebEvaluator;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

use crate::history::WebHistory;

fn init_document_with(document: impl FnOnce(), history: impl FnOnce()) {
    use dioxus_core::has_context;
    Runtime::current().in_scope(ScopeId::ROOT, || {
        if has_context::<Rc<dyn Document>>().is_none() {
            document();
        }
        if has_context::<Rc<dyn History>>().is_none() {
            history();
        }
    })
}

/// Provides the Document through [`dioxus_core::provide_context`].
pub fn init_document() {
    // If hydrate is enabled, we add the FullstackWebDocument with the initial hydration data
    #[cfg(not(feature = "hydrate"))]
    {
        use dioxus_history::provide_history_context;

        init_document_with(
            || {
                provide_context(Rc::new(WebDocument) as Rc<dyn Document>);
            },
            || {
                provide_history_context(Rc::new(WebHistory::default()));
            },
        );
    }
}

#[cfg(feature = "hydrate")]
pub fn init_fullstack_document() {
    use dioxus_fullstack_core::{
        document::FullstackWebDocument, history::provide_fullstack_history_context,
    };

    init_document_with(
        || {
            provide_context(Rc::new(FullstackWebDocument::from(WebDocument)) as Rc<dyn Document>);
        },
        || provide_fullstack_history_context(WebHistory::default()),
    );
}

/// The web-target's document provider.
#[derive(Clone)]
pub struct WebDocument;
impl Document for WebDocument {
    fn eval(&self, js: String) -> Eval {
        Eval::new(WebEvaluator::create(js))
    }

    /// Set the title of the document
    fn set_title(&self, title: String) {
        let myself = self.clone();
        queue_effect(move || {
            myself.eval(format!("document.title = {title:?};"));
        });
    }

    /// Create a new meta tag in the head
    fn create_meta(&self, props: MetaProps) {
        queue_effect(move || {
            _ = append_element_to_head("meta", &props.attributes(), None);
        });
    }

    /// Create a new script tag in the head
    fn create_script(&self, props: ScriptProps) {
        queue_effect(move || {
            _ = append_element_to_head(
                "script",
                &props.attributes(),
                props.script_contents().ok().as_deref(),
            );
        });
    }

    /// Create a new style tag in the head
    fn create_style(&self, props: StyleProps) {
        queue_effect(move || {
            _ = append_element_to_head(
                "style",
                &props.attributes(),
                props.style_contents().ok().as_deref(),
            );
        });
    }

    /// Create a new link tag in the head
    fn create_link(&self, props: LinkProps) {
        queue_effect(move || {
            _ = append_element_to_head("link", &props.attributes(), None);
        });
    }
}

fn append_element_to_head(
    local_name: &str,
    attributes: &Vec<(&'static str, String)>,
    text_content: Option<&str>,
) -> Result<(), JsValue> {
    let window = web_sys_x::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let head = document.head().expect("document should have a head");

    let element = document.create_element(local_name)?;
    for (name, value) in attributes {
        element.set_attribute(name, value)?;
    }
    if text_content.is_some() {
        element.set_text_content(text_content);
    }
    head.append_child(&element)?;

    Ok(())
}
