//! Declarations for the `head` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Head;

/// Build a
/// [`<head>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/head)
/// element.
pub fn head(cx: &ScopeState) -> ElementBuilder<Head> {
    ElementBuilder::new(cx, Head, "head")
}

