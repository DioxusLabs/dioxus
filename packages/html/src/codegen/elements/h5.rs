//! Declarations for the `h5` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct H5;

/// Build a
/// [`<h5>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h5)
/// element.
pub fn h5(cx: &ScopeState) -> ElementBuilder<H5> {
    ElementBuilder::new(cx, H5, "h5")
}

