//! Declarations for the `tfoot` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Tfoot;

/// Build a
/// [`<tfoot>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/tfoot)
/// element.
pub fn tfoot(cx: &ScopeState) -> ElementBuilder<Tfoot> {
    ElementBuilder::new(cx, Tfoot, "tfoot")
}

