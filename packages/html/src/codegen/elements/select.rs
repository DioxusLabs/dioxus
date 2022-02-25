//! Declarations for the `select` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Select;

/// Build a
/// [`<select>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/select)
/// element.
pub fn select(cx: &ScopeState) -> ElementBuilder<Select> {
    ElementBuilder::new(cx, Select, "select")
}

impl<'a> ElementBuilder<'a, Select> {
    #[inline]
    pub fn multiple(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("multiple", val);
        self
    }
    #[inline]
    pub fn form(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("form", val);
        self
    }
    #[inline]
    pub fn autocomplete(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("autocomplete", val);
        self
    }
    #[inline]
    pub fn disabled(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("disabled", val);
        self
    }
    #[inline]
    pub fn autofocus(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("autofocus", val);
        self
    }
    #[inline]
    pub fn required(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("required", val);
        self
    }
    #[inline]
    pub fn size(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("size", val);
        self
    }
    #[inline]
    pub fn name(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("name", val);
        self
    }
} 
