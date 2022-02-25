//! Declarations for the `tr` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Tr;

/// Build a
/// [`<tr>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/tr)
/// element.
pub fn tr(cx: &ScopeState) -> ElementBuilder<Tr> {
    ElementBuilder::new(cx, Tr, "tr")
}

