//! Declarations for the `track` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Track;

/// Build a
/// [`<track>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/track)
/// element.
pub fn track(cx: &ScopeState) -> ElementBuilder<Track> {
    ElementBuilder::new(cx, Track, "track")
}

impl<'a> ElementBuilder<'a, Track> {
    #[inline]
    pub fn src(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("src", val);
        self
    }
    #[inline]
    pub fn srclang(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("srclang", val);
        self
    }
    #[inline]
    pub fn kind(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("kind", val);
        self
    }
    #[inline]
    pub fn label(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("label", val);
        self
    }
    #[inline]
    pub fn default(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("default", val);
        self
    }
} 
