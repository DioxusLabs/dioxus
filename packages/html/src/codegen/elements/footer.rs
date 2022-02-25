//! Declarations for the `footer` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Footer;

/// Build a
/// [`<footer>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/footer)
/// element.
pub fn footer(cx: &ScopeState) -> ElementBuilder<Footer> {
    ElementBuilder::new(cx, Footer, "footer")
}

