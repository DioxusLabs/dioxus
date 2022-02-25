//! Declarations for the `s` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct S;

/// Build a
/// [`<s>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/s)
/// element.
pub fn s(cx: &ScopeState) -> ElementBuilder<S> {
    ElementBuilder::new(cx, S, "s")
}

