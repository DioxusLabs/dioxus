#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(all(feature = "web", feature = "document"))]
mod web;

mod error;

#[cfg(all(feature = "web", feature = "document"))]
pub use web::FullstackWebDocument;

#[cfg(feature = "server")]
pub use dioxus_server::*;

pub use dioxus_fullstack_hooks::history::FullstackHistory;

/// A prelude of commonly used items in dioxus-fullstack.
pub mod prelude {
    pub use dioxus_fullstack_hooks::prelude::*;

    pub use crate::error::{ServerFnError, ServerFnResult};
    pub use dioxus_server_macro::*;
    pub use server_fn::{self, ServerFn as _};

    #[cfg(feature = "server")]
    pub use dioxus_server::prelude::*;
}
