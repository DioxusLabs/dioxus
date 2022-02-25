//! Declarations for the `colgroup` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Colgroup;

/// Build a
/// [`<colgroup>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/colgroup)
/// element.
pub fn colgroup(cx: &ScopeState) -> ElementBuilder<Colgroup> {
    ElementBuilder::new(cx, Colgroup, "colgroup")
}

impl<'a> ElementBuilder<'a, Colgroup> {
    #[inline]
    pub fn span(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("span", val);
        self
    }
} 
