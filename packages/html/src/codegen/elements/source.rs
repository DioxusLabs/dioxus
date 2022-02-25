//! Declarations for the `source` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Source;

/// Build a
/// [`<source>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/source)
/// element.
pub fn source(cx: &ScopeState) -> ElementBuilder<Source> {
    ElementBuilder::new(cx, Source, "source")
}

impl<'a> ElementBuilder<'a, Source> {
    #[inline]
    pub fn r#type(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("type", val);
        self
    }
    #[inline]
    pub fn src(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("src", val);
        self
    }
} 
