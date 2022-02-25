//! Declarations for the `ins` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Ins;

/// Build a
/// [`<ins>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ins)
/// element.
pub fn ins(cx: &ScopeState) -> ElementBuilder<Ins> {
    ElementBuilder::new(cx, Ins, "ins")
}

impl<'a> ElementBuilder<'a, Ins> {
    #[inline]
    pub fn datetime(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("datetime", val);
        self
    }
    #[inline]
    pub fn cite(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("cite", val);
        self
    }
} 
