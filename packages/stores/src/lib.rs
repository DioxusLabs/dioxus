#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![warn(missing_docs)]
#![allow(clippy::type_complexity)]

mod impls;
mod store;
mod subscriptions;
pub use dioxus_stores_macro::Store;
pub use store::*;
mod scope;
pub use scope::SelectorScope;

/// Re-exports for the store derive macro
#[doc(hidden)]
pub mod macro_helpers {
    pub use dioxus_core;
    pub use dioxus_signals;
}
