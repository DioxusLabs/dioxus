//! Declarations for the `option` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Option;

/// Build a
/// [`<option>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/option)
/// element.
pub fn option(cx: &ScopeState) -> ElementBuilder<Option> {
    ElementBuilder::new(cx, Option, "option")
}

impl<'a> ElementBuilder<'a, Option> {
    #[inline]
    pub fn label(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("label", val);
        self
    }
    #[inline]
    pub fn disabled(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("disabled", val);
        self
    }
    #[inline]
    pub fn value(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("value", val);
        self
    }
} 
