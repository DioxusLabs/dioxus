//! Declarations for the `area` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Area;

/// Build a
/// [`<area>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/area)
/// element.
pub fn area(cx: &ScopeState) -> ElementBuilder<Area> {
    ElementBuilder::new(cx, Area, "area")
}

impl<'a> ElementBuilder<'a, Area> {
    #[inline]
    pub fn coords(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("coords", val);
        self
    }
    #[inline]
    pub fn alt(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("alt", val);
        self
    }
    #[inline]
    pub fn hreflang(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("hreflang", val);
        self
    }
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
    #[inline]
    pub fn shape(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("shape", val);
        self
    }
    #[inline]
    pub fn download(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("download", val);
        self
    }
} 
