//! Declarations for the `pre` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Pre;

/// Build a
/// [`<pre>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/pre)
/// element.
pub fn pre(cx: &ScopeState) -> ElementBuilder<Pre> {
    ElementBuilder::new(cx, Pre, "pre")
}

