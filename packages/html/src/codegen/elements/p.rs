//! Declarations for the `p` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct P;

/// Build a
/// [`<p>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/p)
/// element.
pub fn p(cx: &ScopeState) -> ElementBuilder<P> {
    ElementBuilder::new(cx, P, "p")
}

