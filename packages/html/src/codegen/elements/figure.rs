//! Declarations for the `figure` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Figure;

/// Build a
/// [`<figure>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/figure)
/// element.
pub fn figure(cx: &ScopeState) -> ElementBuilder<Figure> {
    ElementBuilder::new(cx, Figure, "figure")
}

