//! Declarations for the `h3` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct H3;

/// Build a
/// [`<h3>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h3)
/// element.
pub fn h3(cx: &ScopeState) -> ElementBuilder<H3> {
    ElementBuilder::new(cx, H3, "h3")
}

