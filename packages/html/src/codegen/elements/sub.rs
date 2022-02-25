//! Declarations for the `sub` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Sub;

/// Build a
/// [`<sub>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/sub)
/// element.
pub fn sub(cx: &ScopeState) -> ElementBuilder<Sub> {
    ElementBuilder::new(cx, Sub, "sub")
}

