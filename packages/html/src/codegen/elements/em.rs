//! Declarations for the `em` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Em;

/// Build a
/// [`<em>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/em)
/// element.
pub fn em(cx: &ScopeState) -> ElementBuilder<Em> {
    ElementBuilder::new(cx, Em, "em")
}

