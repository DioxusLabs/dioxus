//! Declarations for the `i` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct I;

/// Build a
/// [`<i>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/i)
/// element.
pub fn i(cx: &ScopeState) -> ElementBuilder<I> {
    ElementBuilder::new(cx, I, "i")
}

