//! Declarations for the `object` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Object;

/// Build a
/// [`<object>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/object)
/// element.
pub fn object(cx: &ScopeState) -> ElementBuilder<Object> {
    ElementBuilder::new(cx, Object, "object")
}

impl<'a> ElementBuilder<'a, Object> {
    #[inline]
    pub fn name(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("name", val);
        self
    }
    #[inline]
    pub fn usemap(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("usemap", val);
        self
    }
    #[inline]
    pub fn width_(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("width", val);
        self
    }
    #[inline]
    pub fn height_(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("height", val);
        self
    }
    #[inline]
    pub fn form(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("form", val);
        self
    }
    #[inline]
    pub fn r#type(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("type", val);
        self
    }
    #[inline]
    pub fn typemustmatch(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("typemustmatch", val);
        self
    }
}
