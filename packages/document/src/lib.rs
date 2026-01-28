use std::rc::Rc;

mod document;
mod elements;
mod error;
mod eval;
pub mod builder;

pub use document::*;
pub use elements::*;
pub use error::*;
pub use eval::*;

/// Get the document provider for the current platform or a no-op provider if the platform doesn't document functionality.
pub fn document() -> Rc<dyn Document> {
    match dioxus_core::try_consume_context::<Rc<dyn Document>>() {
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
