//! # Dioxus Builder
//!
//! A GPUI-style typed builder API for Dioxus.
//!
//! This crate provides a fluent builder interface for constructing HTML elements
//! with full IDE autocomplete support.
//!
//! ## Example
//!
//! ```rust,ignore
//! use dioxus_builder::*;
//!
//! fn app() -> Element {
//!     div()
//!         .class("container mx-auto")
//!         .id("main")
//!         .onclick(|_| println!("clicked!"))
//!         .child("Hello, World!")
//!         .child(
//!             button()
//!                 .class("btn btn-primary")
//!                 .disabled(true)
//!                 .child("Click me")
//!         )
//!         .build()
//! }
//! ```

mod element;

pub use element::*;
