//! Declarations for the `hr` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Hr;

/// Build a
/// [`<hr>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/hr)
/// element.
pub fn hr(cx: &ScopeState) -> ElementBuilder<Hr> {
    ElementBuilder::new(cx, Hr, "hr")
}

