//! Declarations for the `style` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Style;

/// Build a
/// [`<style>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/style)
/// element.
pub fn style(cx: &ScopeState) -> ElementBuilder<Style> {
    ElementBuilder::new(cx, Style, "style")
}

impl<'a> ElementBuilder<'a, Style> {
    #[inline]
    pub fn r#type(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("type", val);
        self
    }
    #[inline]
    pub fn nonce(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("nonce", val);
        self
    }
    #[inline]
    pub fn media(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("media", val);
        self
    }
} 
