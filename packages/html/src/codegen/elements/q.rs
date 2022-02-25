//! Declarations for the `q` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Q;

/// Build a
/// [`<q>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/q)
/// element.
pub fn q(cx: &ScopeState) -> ElementBuilder<Q> {
    ElementBuilder::new(cx, Q, "q")
}

impl<'a> ElementBuilder<'a, Q> {
    #[inline]
    pub fn cite(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("cite", val);
        self
    }
} 
