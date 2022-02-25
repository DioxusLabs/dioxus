//! Declarations for the `bdo` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Bdo;

/// Build a
/// [`<bdo>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/bdo)
/// element.
pub fn bdo(cx: &ScopeState) -> ElementBuilder<Bdo> {
    ElementBuilder::new(cx, Bdo, "bdo")
}

