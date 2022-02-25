//! Declarations for the `wbr` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Wbr;

/// Build a
/// [`<wbr>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/wbr)
/// element.
pub fn wbr(cx: &ScopeState) -> ElementBuilder<Wbr> {
    ElementBuilder::new(cx, Wbr, "wbr")
}

