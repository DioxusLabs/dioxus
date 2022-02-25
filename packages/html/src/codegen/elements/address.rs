//! Declarations for the `address` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Address;

/// Build a
/// [`<address>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/address)
/// element.
pub fn address(cx: &ScopeState) -> ElementBuilder<Address> {
    ElementBuilder::new(cx, Address, "address")
}

