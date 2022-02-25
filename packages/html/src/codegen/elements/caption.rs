//! Declarations for the `caption` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Caption;

/// Build a
/// [`<caption>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/caption)
/// element.
pub fn caption(cx: &ScopeState) -> ElementBuilder<Caption> {
    ElementBuilder::new(cx, Caption, "caption")
}

