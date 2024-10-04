use std::rc::Rc;

mod document;
mod elements;
mod error;
mod eval;

pub use document::*;
pub use elements::*;
pub use error::*;
pub use eval::*;

/// Get the document provider for the current platform or a no-op provider if the platform doesn't document functionality.
pub fn document() -> Rc<dyn Document> {
    dioxus_core::prelude::try_consume_context::<Rc<dyn Document>>()
        .expect("A document should exist with this renderer")
}

/// Evaluate some javascript in the current document
#[doc = include_str!("../docs/eval.md")]
#[doc(alias = "javascript")]
pub fn eval(script: &str) -> Eval {
    document().eval(script.to_string())
}
