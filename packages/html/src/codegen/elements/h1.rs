//! Declarations for the `h1` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct H1;

/// Build a
/// [`<h1>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h1)
/// element.
pub fn h1(cx: &ScopeState) -> ElementBuilder<H1> {
    ElementBuilder::new(cx, H1, "h1")
}

