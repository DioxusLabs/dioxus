//! Declarations for the `ruby` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Ruby;

/// Build a
/// [`<ruby>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ruby)
/// element.
pub fn ruby(cx: &ScopeState) -> ElementBuilder<Ruby> {
    ElementBuilder::new(cx, Ruby, "ruby")
}

