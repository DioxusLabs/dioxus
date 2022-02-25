//! Declarations for the `nav` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Nav;

/// Build a
/// [`<nav>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/nav)
/// element.
pub fn nav(cx: &ScopeState) -> ElementBuilder<Nav> {
    ElementBuilder::new(cx, Nav, "nav")
}

