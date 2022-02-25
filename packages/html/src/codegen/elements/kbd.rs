//! Declarations for the `kbd` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Kbd;

/// Build a
/// [`<kbd>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/kbd)
/// element.
pub fn kbd(cx: &ScopeState) -> ElementBuilder<Kbd> {
    ElementBuilder::new(cx, Kbd, "kbd")
}

