//! Declarations for the `li` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Li;

/// Build a
/// [`<li>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/li)
/// element.
pub fn li(cx: &ScopeState) -> ElementBuilder<Li> {
    ElementBuilder::new(cx, Li, "li")
}

impl<'a> ElementBuilder<'a, Li> {
    #[inline]
    pub fn value(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("value", val);
        self
    }
} 
