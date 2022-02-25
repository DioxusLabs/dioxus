//! Declarations for the `slot` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Slot;

/// Build a
/// [`<slot>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/slot)
/// element.
pub fn slot(cx: &ScopeState) -> ElementBuilder<Slot> {
    ElementBuilder::new(cx, Slot, "slot")
}

