//! Declarations for the `bdi` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Bdi;

/// Build a
/// [`<bdi>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/bdi)
/// element.
pub fn bdi(cx: &ScopeState) -> ElementBuilder<Bdi> {
    ElementBuilder::new(cx, Bdi, "bdi")
}

