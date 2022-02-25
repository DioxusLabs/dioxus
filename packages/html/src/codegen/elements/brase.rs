//! Declarations for the `brase` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Brase;

/// Build a
/// [`<brase>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/brase)
/// element.
pub fn brase(cx: &ScopeState) -> ElementBuilder<Brase> {
    ElementBuilder::new(cx, Brase, "brase")
}

impl<'a> ElementBuilder<'a, Brase> {
    #[inline]
    pub fn target(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("target", val);
        self
    }
    #[inline]
    pub fn href(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("href", val);
        self
    }
} 
