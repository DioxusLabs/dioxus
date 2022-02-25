//! Declarations for the `rp` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Rp;

/// Build a
/// [`<rp>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/rp)
/// element.
pub fn rp(cx: &ScopeState) -> ElementBuilder<Rp> {
    ElementBuilder::new(cx, Rp, "rp")
}

