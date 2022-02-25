//! Declarations for the `sup` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Sup;

/// Build a
/// [`<sup>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/sup)
/// element.
pub fn sup(cx: &ScopeState) -> ElementBuilder<Sup> {
    ElementBuilder::new(cx, Sup, "sup")
}

