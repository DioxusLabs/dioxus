//! Parse the root tokens in the rsx!{} macro
//! =========================================
//!
//! This parsing path emerges directly from the macro call, with `RsxRender` being the primary entrance into parsing.
//! This feature must support:
//! - [x] Optionally rendering if the `in XYZ` pattern is present
//! - [x] Fragments as top-level element (through ambiguous)
//! - [x] Components as top-level element (through ambiguous)
//! - [x] Tags as top-level elements (through ambiguous)
//! - [x] Good errors if parsing fails
//!
//! Any errors in using rsx! will likely occur when people start using it, so the first errors must be really helpful.

mod ambiguous;
mod body;
mod component;
mod element;
mod fragment;
mod node;

// Re-export the namespaces into each other
pub use ambiguous::*;
pub use body::*;
pub use component::*;
pub use element::*;
pub use fragment::*;
pub use node::*;

pub type HTML_OR_RSX = bool;
pub const AS_HTML: bool = true;
pub const AS_RSX: bool = false;
