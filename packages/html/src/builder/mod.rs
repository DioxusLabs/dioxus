//! # Dioxus Builder
//!
//! A GPUI-style typed builder API for Dioxus.
//!
//! This module provides a fluent builder interface for constructing HTML elements
//! with full IDE autocomplete support.
//!
//! ## Example
//!
//! ```rust,ignore
//! use dioxus::prelude::*;
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
//!
//! ## Static vs Dynamic Children
//!
//! For optimal performance, use `.static_text()` for text that never changes:
//!
//! ```rust,ignore
//! div()
//!     .static_text("Label: ")     // Embedded in template, not diffed
//!     .child(dynamic_value)        // Dynamic, will be diffed
//!     .static_text("!")            // Embedded in template, not diffed
//!     .build()
//! ```
//!
//! ## Document Helpers
//!
//! ```rust,ignore
//! use dioxus::prelude::*;
//!
//! fn app() -> Element {
//!     fragment()
//!         .child(doc_title("My App"))
//!         .child(doc_stylesheet("/assets/style.css"))
//!         .child(body_content())
//!         .build()
//! }
//! ```

mod element;
mod component;

pub use element::*;
pub use component::*;

/// Prelude of builder helpers.
pub mod prelude {
    pub use super::component::*;
    pub use super::element::*;
}
