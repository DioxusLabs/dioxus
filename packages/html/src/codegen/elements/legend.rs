//! Declarations for the `legend` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Legend;

/// Build a
/// [`<legend>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/legend)
/// element.
pub fn legend(cx: &ScopeState) -> ElementBuilder<Legend> {
    ElementBuilder::new(cx, Legend, "legend")
}

