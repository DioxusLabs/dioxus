//! Declarations for the `datalist` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Datalist;

/// Build a
/// [`<datalist>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/datalist)
/// element.
pub fn datalist(cx: &ScopeState) -> ElementBuilder<Datalist> {
    ElementBuilder::new(cx, Datalist, "datalist")
}

