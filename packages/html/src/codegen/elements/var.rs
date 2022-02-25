//! Declarations for the `var` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Var;

/// Build a
/// [`<var>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/var)
/// element.
pub fn var(cx: &ScopeState) -> ElementBuilder<Var> {
    ElementBuilder::new(cx, Var, "var")
}

