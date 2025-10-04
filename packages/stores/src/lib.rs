#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![warn(missing_docs)]
#![allow(clippy::type_complexity)]

mod impls;
mod store;
mod subscriptions;
pub use impls::*;
pub use store::*;
pub mod scope;

#[cfg(feature = "macro")]
pub use dioxus_stores_macro::{store, Store};

/// Re-exports for the store derive macro
#[doc(hidden)]
pub mod macro_helpers {
    pub use dioxus_core;
    pub use dioxus_signals;
}
