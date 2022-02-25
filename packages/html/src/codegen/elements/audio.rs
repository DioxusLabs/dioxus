//! Declarations for the `audio` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Audio;

/// Build a
/// [`<audio>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/audio)
/// element.
pub fn audio(cx: &ScopeState) -> ElementBuilder<Audio> {
    ElementBuilder::new(cx, Audio, "audio")
}

impl<'a> ElementBuilder<'a, Audio> {
    #[inline]
    pub fn controls(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("controls", val);
        self
    }
    #[inline]
    pub fn autoplay(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("autoplay", val);
        self
    }
    #[inline]
    pub fn muted(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("muted", val);
        self
    }
    #[inline]
    pub fn src(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("src", val);
        self
    }
    #[inline]
    pub fn crossorigin(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("crossorigin", val);
        self
    }
    #[inline]
    pub fn preload(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("preload", val);
        self
    }
    #[inline]
    pub fn r#loop(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("loop", val);
        self
    }
} 
