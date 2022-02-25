//! Declarations for the `dt` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Dt;

/// Build a
/// [`<dt>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dt)
/// element.
pub fn dt(cx: &ScopeState) -> ElementBuilder<Dt> {
    ElementBuilder::new(cx, Dt, "dt")
}

