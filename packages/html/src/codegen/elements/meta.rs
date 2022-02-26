//! Declarations for the `meta` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Meta;

/// Build a
/// [`<meta>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meta)
/// element.
pub fn meta(cx: &ScopeState) -> ElementBuilder<Meta> {
    ElementBuilder::new(cx, Meta, "meta")
}

impl<'a> ElementBuilder<'a, Meta> {
    #[inline]
    pub fn charset(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("charset", val);
        self
    }
    #[inline]
    pub fn name(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("name", val);
        self
    }
    #[inline]
    pub fn http_equiv(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("http_equiv", val);
        self
    }
    #[inline]
    pub fn content_(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("content", val);
        self
    }
}
