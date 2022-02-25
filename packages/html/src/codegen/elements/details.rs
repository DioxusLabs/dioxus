//! Declarations for the `details` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Details;

/// Build a
/// [`<details>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/details)
/// element.
pub fn details(cx: &ScopeState) -> ElementBuilder<Details> {
    ElementBuilder::new(cx, Details, "details")
}

impl<'a> ElementBuilder<'a, Details> {
    #[inline]
    pub fn open(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("open", val);
        self
    }
} 
