//! Declarations for the `title` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Title;

/// Build a
/// [`<title>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/title)
/// element.
pub fn title(cx: &ScopeState) -> ElementBuilder<Title> {
    ElementBuilder::new(cx, Title, "title")
}

