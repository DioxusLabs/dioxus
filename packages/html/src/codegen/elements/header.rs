//! Declarations for the `header` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Header;

/// Build a
/// [`<header>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/header)
/// element.
pub fn header(cx: &ScopeState) -> ElementBuilder<Header> {
    ElementBuilder::new(cx, Header, "header")
}

