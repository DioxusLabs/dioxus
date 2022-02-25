//! Declarations for the `table` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Table;

/// Build a
/// [`<table>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/table)
/// element.
pub fn table(cx: &ScopeState) -> ElementBuilder<Table> {
    ElementBuilder::new(cx, Table, "table")
}

