//! Declarations for the `h6` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct H6;

/// Build a
/// [`<h6>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h6)
/// element.
pub fn h6(cx: &ScopeState) -> ElementBuilder<H6> {
    ElementBuilder::new(cx, H6, "h6")
}

