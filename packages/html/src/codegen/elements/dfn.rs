//! Declarations for the `dfn` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Dfn;

/// Build a
/// [`<dfn>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dfn)
/// element.
pub fn dfn(cx: &ScopeState) -> ElementBuilder<Dfn> {
    ElementBuilder::new(cx, Dfn, "dfn")
}

