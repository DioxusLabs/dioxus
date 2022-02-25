//! Declarations for the `div` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Div;

/// Build a
/// [`<div>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/div)
/// element.
pub fn div(cx: &ScopeState) -> ElementBuilder<Div> {
    ElementBuilder::new(cx, Div, "div")
}

