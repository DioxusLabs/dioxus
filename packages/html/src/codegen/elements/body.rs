//! Declarations for the `body` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Body;

/// Build a
/// [`<body>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/body)
/// element.
pub fn body(cx: &ScopeState) -> ElementBuilder<Body> {
    ElementBuilder::new(cx, Body, "body")
}

