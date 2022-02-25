//! Declarations for the `aside` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Aside;

/// Build a
/// [`<aside>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/aside)
/// element.
pub fn aside(cx: &ScopeState) -> ElementBuilder<Aside> {
    ElementBuilder::new(cx, Aside, "aside")
}

