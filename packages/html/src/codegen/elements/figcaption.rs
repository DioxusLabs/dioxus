//! Declarations for the `figcaption` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Figcaption;

/// Build a
/// [`<figcaption>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/figcaption)
/// element.
pub fn figcaption(cx: &ScopeState) -> ElementBuilder<Figcaption> {
    ElementBuilder::new(cx, Figcaption, "figcaption")
}

