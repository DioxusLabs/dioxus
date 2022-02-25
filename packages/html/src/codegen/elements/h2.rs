//! Declarations for the `h2` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct H2;

/// Build a
/// [`<h2>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h2)
/// element.
pub fn h2(cx: &ScopeState) -> ElementBuilder<H2> {
    ElementBuilder::new(cx, H2, "h2")
}

