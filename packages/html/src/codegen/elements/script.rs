//! Declarations for the `script` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Script;

/// Build a
/// [`<script>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script)
/// element.
pub fn script(cx: &ScopeState) -> ElementBuilder<Script> {
    ElementBuilder::new(cx, Script, "script")
}

impl<'a> ElementBuilder<'a, Script> {
    #[inline]
    pub fn nomodule(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("nomodule", val);
        self
    }
    #[inline]
    pub fn crossorigin(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("crossorigin", val);
        self
    }
    #[inline]
    pub fn src(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("src", val);
        self
    }
    #[inline]
    pub fn defer(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("defer", val);
        self
    }
    #[inline]
    pub fn integrity(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("integrity", val);
        self
    }
    #[inline]
    pub fn nonce(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("nonce", val);
        self
    }
    #[inline]
    pub fn text(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("text", val);
        self
    }
} 
