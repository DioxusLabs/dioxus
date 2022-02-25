//! Declarations for the `button` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Button;

/// Build a
/// [`<button>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/button)
/// element.
pub fn button(cx: &ScopeState) -> ElementBuilder<Button> {
    ElementBuilder::new(cx, Button, "button")
}

impl<'a> ElementBuilder<'a, Button> {
    #[inline]
    pub fn formmethod(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("formmethod", val);
        self
    }
    #[inline]
    pub fn value(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("value", val);
        self
    }
    #[inline]
    pub fn formaction(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("formaction", val);
        self
    }
    #[inline]
    pub fn autofocus(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("autofocus", val);
        self
    }
    #[inline]
    pub fn formenctype(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("formenctype", val);
        self
    }
    #[inline]
    pub fn formnovalidate(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("formnovalidate", val);
        self
    }
    #[inline]
    pub fn form(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("form", val);
        self
    }
    #[inline]
    pub fn formtarget(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("formtarget", val);
        self
    }
    #[inline]
    pub fn name(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("name", val);
        self
    }
    #[inline]
    pub fn disabled(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("disabled", val);
        self
    }
} 
