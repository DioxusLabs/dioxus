//! Declarations for the `thead` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Thead;

/// Build a
/// [`<thead>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/thead)
/// element.
pub fn thead(cx: &ScopeState) -> ElementBuilder<Thead> {
    ElementBuilder::new(cx, Thead, "thead")
}

