//! Declarations for the `b` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct B;

/// Build a
/// [`<b>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/b)
/// element.
pub fn b(cx: &ScopeState) -> ElementBuilder<B> {
    ElementBuilder::new(cx, B, "b")
}

