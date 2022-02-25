//! Declarations for the `link` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Link;

/// Build a
/// [`<link>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link)
/// element.
pub fn link(cx: &ScopeState) -> ElementBuilder<Link> {
    ElementBuilder::new(cx, Link, "link")
}

impl<'a> ElementBuilder<'a, Link> {
    #[inline]
    pub fn crossorigin(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("crossorigin", val);
        self
    }
    #[inline]
    pub fn r#as(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("as", val);
        self
    }
    #[inline]
    pub fn hreflang(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("hreflang", val);
        self
    }
    #[inline]
    pub fn media(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("media", val);
        self
    }
    #[inline]
    pub fn rel(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("rel", val);
        self
    }
    #[inline]
    pub fn sizes(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("sizes", val);
        self
    }
    #[inline]
    pub fn r#type(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("type", val);
        self
    }
    #[inline]
    pub fn href(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("href", val);
        self
    }
    #[inline]
    pub fn integrity(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("integrity", val);
        self
    }
} 
