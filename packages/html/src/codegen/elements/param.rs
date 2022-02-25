//! Declarations for the `param` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Param;

/// Build a
/// [`<param>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/param)
/// element.
pub fn param(cx: &ScopeState) -> ElementBuilder<Param> {
    ElementBuilder::new(cx, Param, "param")
}

impl<'a> ElementBuilder<'a, Param> {
    #[inline]
    pub fn value(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("value", val);
        self
    }
    #[inline]
    pub fn name(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("name", val);
        self
    }
} 
