//! Declarations for the `dd` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Dd;

/// Build a
/// [`<dd>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dd)
/// element.
pub fn dd(cx: &ScopeState) -> ElementBuilder<Dd> {
    ElementBuilder::new(cx, Dd, "dd")
}

