//! Declarations for the `main` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Main;

/// Build a
/// [`<main>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/main)
/// element.
pub fn main(cx: &ScopeState) -> ElementBuilder<Main> {
    ElementBuilder::new(cx, Main, "main")
}

