//! Declarations for the `label` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Label;

/// Build a
/// [`<label>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/label)
/// element.
pub fn label(cx: &ScopeState) -> ElementBuilder<Label> {
    ElementBuilder::new(cx, Label, "label")
}

impl<'a> ElementBuilder<'a, Label> {
    #[inline]
    pub fn form(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("form", val);
        self
    }
    #[inline]
    pub fn r#for(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("for", val);
        self
    }
} 
