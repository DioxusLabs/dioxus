//! Declarations for the `output` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Output;

/// Build a
/// [`<output>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/output)
/// element.
pub fn output(cx: &ScopeState) -> ElementBuilder<Output> {
    ElementBuilder::new(cx, Output, "output")
}

impl<'a> ElementBuilder<'a, Output> {
    #[inline]
    pub fn name(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("name", val);
        self
    }
    #[inline]
    pub fn r#for(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("for", val);
        self
    }
    #[inline]
    pub fn form(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("form", val);
        self
    }
} 
