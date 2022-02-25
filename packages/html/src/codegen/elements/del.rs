//! Declarations for the `del` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Del;

/// Build a
/// [`<del>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/del)
/// element.
pub fn del(cx: &ScopeState) -> ElementBuilder<Del> {
    ElementBuilder::new(cx, Del, "del")
}

impl<'a> ElementBuilder<'a, Del> {
    #[inline]
    pub fn cite(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("cite", val);
        self
    }
    #[inline]
    pub fn datetime(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("datetime", val);
        self
    }
} 
