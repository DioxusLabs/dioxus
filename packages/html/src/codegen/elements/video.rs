//! Declarations for the `video` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Video;

/// Build a
/// [`<video>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/video)
/// element.
pub fn video(cx: &ScopeState) -> ElementBuilder<Video> {
    ElementBuilder::new(cx, Video, "video")
}

impl<'a> ElementBuilder<'a, Video> {
    #[inline]
    pub fn height_(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("height", val);
        self
    }
    #[inline]
    pub fn crossorigin(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("crossorigin", val);
        self
    }
    #[inline]
    pub fn autoplay(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("autoplay", val);
        self
    }
    #[inline]
    pub fn controls(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("controls", val);
        self
    }
    #[inline]
    pub fn muted(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("muted", val);
        self
    }
    #[inline]
    pub fn poster(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("poster", val);
        self
    }
    #[inline]
    pub fn src(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("src", val);
        self
    }
    #[inline]
    pub fn preload(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("preload", val);
        self
    }
    #[inline]
    pub fn width_(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("width", val);
        self
    }
    #[inline]
    pub fn playsinline(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("playsinline", val);
        self
    }
    #[inline]
    pub fn r#loop(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("loop", val);
        self
    }
}
