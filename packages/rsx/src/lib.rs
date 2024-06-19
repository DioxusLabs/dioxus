#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

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
//!
//! # Completions
//! Rust analyzer completes macros by looking at the expansion of the macro and trying to match the start of identifiers in the macro to identifiers in the current scope
//!
//! Eg, if a macro expands to this:
//! ```rust, ignore
//! struct MyStruct;
//!
//! // macro expansion
//! My
//! ```
//! Then the analyzer will try to match the start of the identifier "My" to an identifier in the current scope (MyStruct in this case).
//!
//! In dioxus, our macros expand to the completions module if we know the identifier is incomplete:
//! ```rust, ignore
//! // In the root of the macro, identifiers must be elements
//! // rsx! { di }
//! dioxus_elements::elements::di
//!
//! // Before the first child element, every following identifier is either an attribute or an element
//! // rsx! { div { ta } }
//! // Isolate completions scope
//! mod completions__ {
//!     // import both the attributes and elements this could complete to
//!     use dioxus_elements::elements::div::*;
//!     use dioxus_elements::elements::*;
//!     fn complete() {
//!         ta;
//!     }
//! }
//!
//! // After the first child element, every following identifier is another element
//! // rsx! { div { attribute: value, child {} di } }
//! dioxus_elements::elements::di
//! ```

mod body;
mod diagnostics;
pub mod hotreload;
mod ifmt;
mod location;
mod node;
mod reload_stack;
mod rsx_call;
mod scoring;
mod utils;
// pub(crate) mod context;

// Re-export the namespaces into each other
pub use body::TemplateBody;
pub use diagnostics::Diagnostics;
pub use ifmt::*;
pub use node::*;
pub use rsx_call::*;
pub use utils::*;

#[cfg(feature = "hot_reload")]
pub mod hot_reload;

#[cfg(feature = "hot_reload")]
use dioxus_core::{TemplateAttribute, TemplateNode};
#[cfg(feature = "hot_reload")]
pub use hot_reload::HotReloadingContext;

use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    Result, Token,
};
