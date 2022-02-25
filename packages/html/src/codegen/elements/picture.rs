//! Declarations for the `picture` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Picture;

/// Build a
/// [`<picture>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/picture)
/// element.
pub fn picture(cx: &ScopeState) -> ElementBuilder<Picture> {
    ElementBuilder::new(cx, Picture, "picture")
}

