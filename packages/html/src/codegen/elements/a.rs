//! Declarations for the `a` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct A;

/// Build a
/// [`<a>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/a)
/// element.
pub fn a(cx: &ScopeState) -> ElementBuilder<A> {
    ElementBuilder::new(cx, A, "a")
}

impl<'a> ElementBuilder<'a, A> {
    #[inline]
    pub fn rel(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("rel", val);
        self
    }
    #[inline]
    pub fn target(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("target", val);
        self
    }
    #[inline]
    pub fn hreflang(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("hreflang", val);
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
    pub fn ping(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("ping", val);
        self
    }
    #[inline]
    pub fn download(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("download", val);
        self
    }
} 
