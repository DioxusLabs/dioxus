//! Declarations for the `summary` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Summary;

/// Build a
/// [`<summary>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/summary)
/// element.
pub fn summary(cx: &ScopeState) -> ElementBuilder<Summary> {
    ElementBuilder::new(cx, Summary, "summary")
}

