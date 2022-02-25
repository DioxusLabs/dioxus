//! Declarations for the `strong` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Strong;

/// Build a
/// [`<strong>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/strong)
/// element.
pub fn strong(cx: &ScopeState) -> ElementBuilder<Strong> {
    ElementBuilder::new(cx, Strong, "strong")
}

