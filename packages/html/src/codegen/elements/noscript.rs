//! Declarations for the `noscript` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Noscript;

/// Build a
/// [`<noscript>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/noscript)
/// element.
pub fn noscript(cx: &ScopeState) -> ElementBuilder<Noscript> {
    ElementBuilder::new(cx, Noscript, "noscript")
}

