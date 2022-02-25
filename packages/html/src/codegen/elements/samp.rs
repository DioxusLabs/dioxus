//! Declarations for the `samp` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Samp;

/// Build a
/// [`<samp>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/samp)
/// element.
pub fn samp(cx: &ScopeState) -> ElementBuilder<Samp> {
    ElementBuilder::new(cx, Samp, "samp")
}

