//! Declarations for the `u` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct U;

/// Build a
/// [`<u>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/u)
/// element.
pub fn u(cx: &ScopeState) -> ElementBuilder<U> {
    ElementBuilder::new(cx, U, "u")
}

