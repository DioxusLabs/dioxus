//! Declarations for the `rt` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Rt;

/// Build a
/// [`<rt>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/rt)
/// element.
pub fn rt(cx: &ScopeState) -> ElementBuilder<Rt> {
    ElementBuilder::new(cx, Rt, "rt")
}

