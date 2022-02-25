//! Declarations for the `input` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Input;

/// Build a
/// [`<input>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/input)
/// element.
pub fn input(cx: &ScopeState) -> ElementBuilder<Input> {
    ElementBuilder::new(cx, Input, "input")
}

impl<'a> ElementBuilder<'a, Input> {
    #[inline]
    pub fn alt(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("alt", val);
        self
    }
    #[inline]
    pub fn capture(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("capture", val);
        self
    }
    #[inline]
    pub fn formmethod(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("formmethod", val);
        self
    }
    #[inline]
    pub fn form(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("form", val);
        self
    }
    #[inline]
    pub fn height(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("height", val);
        self
    }
    #[inline]
    pub fn formaction(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("formaction", val);
        self
    }
    #[inline]
    pub fn formenctype(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("formenctype", val);
        self
    }
    #[inline]
    pub fn autofocus(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("autofocus", val);
        self
    }
    #[inline]
    pub fn min(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("min", val);
        self
    }
    #[inline]
    pub fn pattern(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("pattern", val);
        self
    }
    #[inline]
    pub fn readonly(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("readonly", val);
        self
    }
    #[inline]
    pub fn name(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("name", val);
        self
    }
    #[inline]
    pub fn minlength(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("minlength", val);
        self
    }
    #[inline]
    pub fn maxlength(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("maxlength", val);
        self
    }
    #[inline]
    pub fn multiple(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("multiple", val);
        self
    }
    #[inline]
    pub fn size(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("size", val);
        self
    }
    #[inline]
    pub fn autocomplete(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("autocomplete", val);
        self
    }
    #[inline]
    pub fn checked(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("checked", val);
        self
    }
    #[inline]
    pub fn r#type(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("type", val);
        self
    }
    #[inline]
    pub fn step(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("step", val);
        self
    }
    #[inline]
    pub fn placeholder(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("placeholder", val);
        self
    }
    #[inline]
    pub fn max(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("max", val);
        self
    }
    #[inline]
    pub fn list(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("list", val);
        self
    }
    #[inline]
    pub fn width(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("width", val);
        self
    }
    #[inline]
    pub fn src(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("src", val);
        self
    }
    #[inline]
    pub fn disabled(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("disabled", val);
        self
    }
    #[inline]
    pub fn formnovalidate(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("formnovalidate", val);
        self
    }
    #[inline]
    pub fn formtarget(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("formtarget", val);
        self
    }
    #[inline]
    pub fn accept(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("accept", val);
        self
    }
    #[inline]
    pub fn required(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("required", val);
        self
    }
} 
