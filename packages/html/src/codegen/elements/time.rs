//! Declarations for the `time` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Time;

/// Build a
/// [`<time>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/time)
/// element.
pub fn time(cx: &ScopeState) -> ElementBuilder<Time> {
    ElementBuilder::new(cx, Time, "time")
}

