//! Declarations for the `span` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Span;

/// Build a
/// [`<span>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/span)
/// element.
pub fn span(cx: &ScopeState) -> ElementBuilder<Span> {
    ElementBuilder::new(cx, Span, "span")
}

