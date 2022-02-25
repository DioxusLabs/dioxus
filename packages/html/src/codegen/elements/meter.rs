//! Declarations for the `meter` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Meter;

/// Build a
/// [`<meter>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meter)
/// element.
pub fn meter(cx: &ScopeState) -> ElementBuilder<Meter> {
    ElementBuilder::new(cx, Meter, "meter")
}

impl<'a> ElementBuilder<'a, Meter> {
    #[inline]
    pub fn form(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("form", val);
        self
    }
    #[inline]
    pub fn optimum(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("optimum", val);
        self
    }
    #[inline]
    pub fn max(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("max", val);
        self
    }
    #[inline]
    pub fn low(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("low", val);
        self
    }
    #[inline]
    pub fn high(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("high", val);
        self
    }
    #[inline]
    pub fn value(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("value", val);
        self
    }
    #[inline]
    pub fn min(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("min", val);
        self
    }
} 
