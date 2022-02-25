//! Declarations for the `base` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Base;

/// Build a
/// [`<base>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/base)
/// element.
pub fn base(cx: &ScopeState) -> ElementBuilder<Base> {
    ElementBuilder::new(cx, Base, "base")
}

impl<'a> ElementBuilder<'a, Base> {
    #[inline]
    pub fn target(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("target", val);
        self
    }
    #[inline]
    pub fn href(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("href", val);
        self
    }
} 
