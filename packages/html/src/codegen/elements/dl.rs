//! Declarations for the `dl` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Dl;

/// Build a
/// [`<dl>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dl)
/// element.
pub fn dl(cx: &ScopeState) -> ElementBuilder<Dl> {
    ElementBuilder::new(cx, Dl, "dl")
}

