use std::rc::Rc;

mod document;
mod eval;
mod head;
mod title;

pub use document::*;
pub use eval::*;
pub use head::*;
pub use title::*;

/// Get the document provider for the current platform or a no-op provider if the platform doesn't document functionality.
pub fn document() -> Rc<dyn Document> {
    dioxus_core::prelude::try_consume_context::<Rc<dyn Document>>()
        .expect("A document should exist with this renderer")
}
