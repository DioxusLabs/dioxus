//! Declarations for the `embed` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Embed;

/// Build a
/// [`<embed>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/embed)
/// element.
pub fn embed(cx: &ScopeState) -> ElementBuilder<Embed> {
    ElementBuilder::new(cx, Embed, "embed")
}

impl<'a> ElementBuilder<'a, Embed> {
    #[inline]
    pub fn r#type(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("type", val);
        self
    }
    #[inline]
    pub fn height(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("height", val);
        self
    }
    #[inline]
    pub fn src(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("src", val);
        self
    }
    #[inline]
    pub fn width(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("width", val);
        self
    }
} 
