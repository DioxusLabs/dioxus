//! Declarations for the `small` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Small;

/// Build a
/// [`<small>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/small)
/// element.
pub fn small(cx: &ScopeState) -> ElementBuilder<Small> {
    ElementBuilder::new(cx, Small, "small")
}

