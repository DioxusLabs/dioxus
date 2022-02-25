//! Declarations for the `ul` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Ul;

/// Build a
/// [`<ul>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ul)
/// element.
pub fn ul(cx: &ScopeState) -> ElementBuilder<Ul> {
    ElementBuilder::new(cx, Ul, "ul")
}

