#![cfg_attr(docsrs, feature(doc_cfg))]
// TODO: Turn this into real text
//! A testing library for Dioxus.
//!
//! - Facilitates rendering, interacting with, and querying the DOM.
//! - Completely headless.
//! - Based on dioxus-native crate, using Blitz to layout.
//! - Much more flexible than using dioxus-ssr (which doesn't allow interaction) and much easier to
//!   write tests than Playwright.
//! - Allows fairly precise control over asynchronous operations, so tests can assert on the state
//!   while async operations are in progress.
//!
//! ## Usage
//!
//! - How to build a Tester
//! - How to query and interact with elements
//! - How to assert on elements
//!
//! ## Asynchronous operations
//!
//! - Asserting on "in flight" asynchronous operations
//! - Resolving asynchronous operations
//!
//! ## Limitations
//!
//! - Interactions operate directly on elements, not on the screen. So if, say, you dispatch a click
//!   on an element which is covered by a frost, the element will respond as though it were
//!   reachable even though it would not be in reality.
//! - Limited by what the Blitz layout system can support. Layouts might not be as in reality.

mod element;
mod tester;

pub use element::TestElement;
pub use tester::{Tester, TesterError, render};
