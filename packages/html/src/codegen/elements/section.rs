//! Declarations for the `section` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Section;

/// Build a
/// [`<section>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/section)
/// element.
pub fn section(cx: &ScopeState) -> ElementBuilder<Section> {
    ElementBuilder::new(cx, Section, "section")
}

