use std::rc::Rc;

mod document;
mod elements;
mod error;
mod eval;
mod listener;

pub use document::*;
pub use elements::*;
pub use error::*;
pub use eval::*;
pub use listener::{
    Containment, GlobalEventCallback, GlobalEventInput, GlobalEventPayload, GlobalListenerContext,
    GlobalListenerOptions, GlobalListenerRegistration, GlobalListenerSupport, ViewportResizeData,
};

#[doc(hidden)]
pub mod __private {
    pub use crate::listener::{use_document_listener, use_window_listener};
    pub use dioxus_html::events::__private::global_events;
    pub mod window_events {
        pub mod plain {
            pub use dioxus_html::events::__private::{
                global_events::plain::*, viewport_resize::plain::*,
            };
        }
        pub mod contextual {
            pub use dioxus_html::events::__private::{
                global_events::contextual::*, viewport_resize::contextual::*,
            };
        }
    }
}

/// Get the document provider for the current platform or a no-op provider if the platform doesn't document functionality.
pub fn document() -> Rc<dyn Document> {
    match dioxus_vdom::try_consume_context::<Rc<dyn Document>>() {
        Some(document) => document,
        None => {
            tracing::error!(
                "Unable to find a document in the renderer. Using the default no-op document."
            );
            Rc::new(NoOpDocument)
        }
    }
}

/// Evaluate some javascript in the current document
#[doc = include_str!("../docs/eval.md")]
#[doc(alias = "javascript")]
pub fn eval(script: &str) -> Eval {
    document().eval(script.to_string())
}
