//! Declarations for the `tbody` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Tbody;

/// Build a
/// [`<tbody>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/tbody)
/// element.
pub fn tbody(cx: &ScopeState) -> ElementBuilder<Tbody> {
    ElementBuilder::new(cx, Tbody, "tbody")
}

