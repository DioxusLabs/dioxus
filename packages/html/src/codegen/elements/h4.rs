//! Declarations for the `h4` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct H4;

/// Build a
/// [`<h4>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h4)
/// element.
pub fn h4(cx: &ScopeState) -> ElementBuilder<H4> {
    ElementBuilder::new(cx, H4, "h4")
}

