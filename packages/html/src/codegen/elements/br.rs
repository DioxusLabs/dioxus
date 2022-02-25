//! Declarations for the `br` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Br;

/// Build a
/// [`<br>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/br)
/// element.
pub fn br(cx: &ScopeState) -> ElementBuilder<Br> {
    ElementBuilder::new(cx, Br, "br")
}

