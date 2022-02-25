//! Declarations for the `canvas` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Canvas;

/// Build a
/// [`<canvas>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/canvas)
/// element.
pub fn canvas(cx: &ScopeState) -> ElementBuilder<Canvas> {
    ElementBuilder::new(cx, Canvas, "canvas")
}

impl<'a> ElementBuilder<'a, Canvas> {
    #[inline]
    pub fn width(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("width", val);
        self
    }
    #[inline]
    pub fn height(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("height", val);
        self
    }
} 
