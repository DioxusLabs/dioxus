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

#[macro_use]
mod errors;
mod body;
mod diagnostics;
pub mod hotreload;
mod ifmt;
mod location;
mod node;
mod rsx_call;

// pub(crate) mod context;

// Re-export the namespaces into each other
pub use body::TemplateBody;
// pub use context::{CallBodyContexta, DynamicContext};
pub use diagnostics::Diagnostics;
pub use ifmt::*;
pub use node::*;
pub use rsx_call::*;

#[cfg(feature = "hot_reload")]
pub mod hot_reload;

#[cfg(feature = "hot_reload")]
use dioxus_core::{TemplateAttribute, TemplateNode};
#[cfg(feature = "hot_reload")]
pub use hot_reload::HotReloadingContext;
#[cfg(feature = "hot_reload")]
use internment::Intern;

use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use std::{fmt::Debug, hash::Hash};
use syn::{
    parse::{Parse, ParseStream},
    Result, Token,
};

#[cfg(feature = "hot_reload")]
// interns a object into a static object, resusing the value if it already exists
pub(crate) fn intern<T: Eq + Hash + Send + Sync + ?Sized + 'static>(
    s: impl Into<Intern<T>>,
) -> &'static T {
    s.into().as_ref()
}
