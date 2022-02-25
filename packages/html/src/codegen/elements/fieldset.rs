//! Declarations for the `fieldset` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Fieldset;

/// Build a
/// [`<fieldset>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/fieldset)
/// element.
pub fn fieldset(cx: &ScopeState) -> ElementBuilder<Fieldset> {
    ElementBuilder::new(cx, Fieldset, "fieldset")
}

