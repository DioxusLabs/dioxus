//! Declarations for the `progress` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Progress;

/// Build a
/// [`<progress>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/progress)
/// element.
pub fn progress(cx: &ScopeState) -> ElementBuilder<Progress> {
    ElementBuilder::new(cx, Progress, "progress")
}

impl<'a> ElementBuilder<'a, Progress> {
    #[inline]
    pub fn value(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("value", val);
        self
    }
    #[inline]
    pub fn max(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("max", val);
        self
    }
} 
