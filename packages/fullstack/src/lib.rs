#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub use once_cell;

#[cfg(all(feature = "web", feature = "document"))]
mod web;

#[cfg(all(feature = "web", feature = "document"))]
pub use web::FullstackWebDocument;

/// A prelude of commonly used items in dioxus-fullstack.
pub mod prelude {
    pub use dioxus_fullstack_hooks::*;

    #[cfg(feature = "server")]
    #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
    pub use dioxus_isrg::{IncrementalRenderer, IncrementalRendererConfig};

    pub use dioxus_server_macro::*;
    pub use server_fn::{self, ServerFn as _, ServerFnError};
}
