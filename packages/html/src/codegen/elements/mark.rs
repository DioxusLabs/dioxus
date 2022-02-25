//! Declarations for the `mark` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Mark;

/// Build a
/// [`<mark>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/mark)
/// element.
pub fn mark(cx: &ScopeState) -> ElementBuilder<Mark> {
    ElementBuilder::new(cx, Mark, "mark")
}

