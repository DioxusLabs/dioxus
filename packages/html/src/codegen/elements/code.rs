//! Declarations for the `code` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Code;

/// Build a
/// [`<code>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/code)
/// element.
pub fn code(cx: &ScopeState) -> ElementBuilder<Code> {
    ElementBuilder::new(cx, Code, "code")
}

impl<'a> ElementBuilder<'a, Code> {
    #[inline]
    pub fn language(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("language", val);
        self
    }
} 
