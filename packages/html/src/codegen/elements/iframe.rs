//! Declarations for the `iframe` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Iframe;

/// Build a
/// [`<iframe>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/iframe)
/// element.
pub fn iframe(cx: &ScopeState) -> ElementBuilder<Iframe> {
    ElementBuilder::new(cx, Iframe, "iframe")
}

impl<'a> ElementBuilder<'a, Iframe> {
    #[inline]
    pub fn src(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("src", val);
        self
    }
    #[inline]
    pub fn allow(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("allow", val);
        self
    }
    #[inline]
    pub fn marginHeight(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("marginHeight", val);
        self
    }
    #[inline]
    pub fn frameBorder(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("frameBorder", val);
        self
    }
    #[inline]
    pub fn scrolling(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("scrolling", val);
        self
    }
    #[inline]
    pub fn height(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("height", val);
        self
    }
    #[inline]
    pub fn allowpaymentrequest(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("allowpaymentrequest", val);
        self
    }
    #[inline]
    pub fn width(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("width", val);
        self
    }
    #[inline]
    pub fn allowfullscreen(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("allowfullscreen", val);
        self
    }
    #[inline]
    pub fn srcdoc(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("srcdoc", val);
        self
    }
    #[inline]
    pub fn marginWidth(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("marginWidth", val);
        self
    }
    #[inline]
    pub fn longdesc(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("longdesc", val);
        self
    }
    #[inline]
    pub fn referrerpolicy(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("referrerpolicy", val);
        self
    }
    #[inline]
    pub fn name(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("name", val);
        self
    }
    #[inline]
    pub fn align(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("align", val);
        self
    }
} 
