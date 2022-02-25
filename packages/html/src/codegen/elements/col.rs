//! Declarations for the `col` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Col;

/// Build a
/// [`<col>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/col)
/// element.
pub fn col(cx: &ScopeState) -> ElementBuilder<Col> {
    ElementBuilder::new(cx, Col, "col")
}

impl<'a> ElementBuilder<'a, Col> {
    #[inline]
    pub fn span(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("span", val);
        self
    }
} 
