//! Declarations for the `abbr` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Abbr;

/// Build a
/// [`<abbr>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/abbr)
/// element.
pub fn abbr(cx: &ScopeState) -> ElementBuilder<Abbr> {
    ElementBuilder::new(cx, Abbr, "abbr")
}
