//! Declarations for the `td` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Td;

/// Build a
/// [`<td>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/td)
/// element.
pub fn td(cx: &ScopeState) -> ElementBuilder<Td> {
    ElementBuilder::new(cx, Td, "td")
}

impl<'a> ElementBuilder<'a, Td> {
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
} 
