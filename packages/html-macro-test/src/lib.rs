//! Tests for our html! procedural macro
//!
//! To run all tests in this library:
//!
//! cargo test --color=always --package html-macro-test --lib "" -- --nocapture

// #![feature(proc_macro_hygiene)]

// TODO: Deny warnings to ensure that the macro isn't creating any warnings.
// #![deny(warnings)]

#[cfg(test)]
mod tests;
