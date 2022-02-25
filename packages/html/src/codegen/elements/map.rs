//! Declarations for the `map` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Map;

/// Build a
/// [`<map>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/map)
/// element.
pub fn map(cx: &ScopeState) -> ElementBuilder<Map> {
    ElementBuilder::new(cx, Map, "map")
}

impl<'a> ElementBuilder<'a, Map> {
    #[inline]
    pub fn name(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("name", val);
        self
    }
} 
