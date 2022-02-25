//! Declarations for the `ol` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Ol;

/// Build a
/// [`<ol>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ol)
/// element.
pub fn ol(cx: &ScopeState) -> ElementBuilder<Ol> {
    ElementBuilder::new(cx, Ol, "ol")
}

impl<'a> ElementBuilder<'a, Ol> {
    #[inline]
    pub fn reversed(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("reversed", val);
        self
    }
    #[inline]
    pub fn start(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("start", val);
        self
    }
    #[inline]
    pub fn r#type(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("type", val);
        self
    }
} 
