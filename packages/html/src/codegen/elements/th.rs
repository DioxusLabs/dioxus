//! Declarations for the `th` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Th;

/// Build a
/// [`<th>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/th)
/// element.
pub fn th(cx: &ScopeState) -> ElementBuilder<Th> {
    ElementBuilder::new(cx, Th, "th")
}

impl<'a> ElementBuilder<'a, Th> {
    #[inline]
    pub fn abbr(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("abbr", val);
        self
    }
    #[inline]
    pub fn colspan(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("colspan", val);
        self
    }
    #[inline]
    pub fn rowspan(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("rowspan", val);
        self
    }
    #[inline]
    pub fn scope(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("scope", val);
        self
    }
} 
