//! Declarations for the `img` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Img;

/// Build a
/// [`<img>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/img)
/// element.
pub fn img(cx: &ScopeState) -> ElementBuilder<Img> {
    ElementBuilder::new(cx, Img, "img")
}

impl<'a> ElementBuilder<'a, Img> {
    #[inline]
    pub fn width_(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("width", val);
        self
    }
    #[inline]
    pub fn height_(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("height", val);
        self
    }
    #[inline]
    pub fn srcset(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("srcset", val);
        self
    }
    #[inline]
    pub fn referrerpolicy(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("referrerpolicy", val);
        self
    }
    #[inline]
    pub fn decoding(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("decoding", val);
        self
    }
    #[inline]
    pub fn usemap(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("usemap", val);
        self
    }
    #[inline]
    pub fn ismap(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("ismap", val);
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
    pub fn alt(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("alt", val);
        self
    }
}
