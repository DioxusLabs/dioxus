//! Declarations for the `optgroup` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Optgroup;

/// Build a
/// [`<optgroup>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/optgroup)
/// element.
pub fn optgroup(cx: &ScopeState) -> ElementBuilder<Optgroup> {
    ElementBuilder::new(cx, Optgroup, "optgroup")
}

impl<'a> ElementBuilder<'a, Optgroup> {
    #[inline]
    pub fn disabled(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("disabled", val);
        self
    }
    #[inline]
    pub fn label(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("label", val);
        self
    }
} 
