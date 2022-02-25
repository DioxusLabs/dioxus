//! Declarations for the `lrink` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Lrink;

/// Build a
/// [`<lrink>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/lrink)
/// element.
pub fn lrink(cx: &ScopeState) -> ElementBuilder<Lrink> {
    ElementBuilder::new(cx, Lrink, "lrink")
}

