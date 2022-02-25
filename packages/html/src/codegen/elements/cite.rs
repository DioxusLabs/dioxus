//! Declarations for the `cite` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Cite;

/// Build a
/// [`<cite>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/cite)
/// element.
pub fn cite(cx: &ScopeState) -> ElementBuilder<Cite> {
    ElementBuilder::new(cx, Cite, "cite")
}

