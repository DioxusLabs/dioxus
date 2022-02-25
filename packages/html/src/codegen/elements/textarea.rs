//! Declarations for the `textarea` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Textarea;

/// Build a
/// [`<textarea>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/textarea)
/// element.
pub fn textarea(cx: &ScopeState) -> ElementBuilder<Textarea> {
    ElementBuilder::new(cx, Textarea, "textarea")
}

impl<'a> ElementBuilder<'a, Textarea> {
    #[inline]
    pub fn name(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("name", val);
        self
    }
    #[inline]
    pub fn placeholder(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("placeholder", val);
        self
    }
    #[inline]
    pub fn required(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("required", val);
        self
    }
    #[inline]
    pub fn wrap(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("wrap", val);
        self
    }
    #[inline]
    pub fn autocomplete(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("autocomplete", val);
        self
    }
    #[inline]
    pub fn autofocus(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("autofocus", val);
        self
    }
    #[inline]
    pub fn minlength(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("minlength", val);
        self
    }
    #[inline]
    pub fn cols(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("cols", val);
        self
    }
    #[inline]
    pub fn readonly(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("readonly", val);
        self
    }
    #[inline]
    pub fn maxlength(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("maxlength", val);
        self
    }
    #[inline]
    pub fn rows(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("rows", val);
        self
    }
    #[inline]
    pub fn form(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("form", val);
        self
    }
    #[inline]
    pub fn disabled(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("disabled", val);
        self
    }
} 
