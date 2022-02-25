//! Declarations for the `blockquote` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Blockquote;

/// Build a
/// [`<blockquote>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/blockquote)
/// element.
pub fn blockquote(cx: &ScopeState) -> ElementBuilder<Blockquote> {
    ElementBuilder::new(cx, Blockquote, "blockquote")
}

impl<'a> ElementBuilder<'a, Blockquote> {
    #[inline]
    pub fn cite(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("cite", val);
        self
    }
} 
