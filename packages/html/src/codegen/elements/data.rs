//! Declarations for the `data` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Data;

/// Build a
/// [`<data>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/data)
/// element.
pub fn data(cx: &ScopeState) -> ElementBuilder<Data> {
    ElementBuilder::new(cx, Data, "data")
}

impl<'a> ElementBuilder<'a, Data> {
    #[inline]
    pub fn value(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("value", val);
        self
    }
} 
