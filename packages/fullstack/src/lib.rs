#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub use once_cell;

#[cfg(all(feature = "web"))]
mod document;
mod html_storage;
mod server_cached;
mod server_future;

/// A prelude of commonly used items in dioxus-fullstack.
pub mod prelude {
    use super::*;

    pub use server_cached::use_server_cached;
    pub use server_future::use_server_future;

    // #[cfg(feature = "axum")]
    // #[cfg_attr(docsrs, doc(cfg(feature = "axum")))]
    // pub use crate::server::*;

    // #[cfg(feature = "server")]
    // #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
    // pub use crate::render::{FullstackHTMLTemplate, SSRState};

    // #[cfg(feature = "server")]
    // #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
    // pub use crate::serve_config::{ServeConfig, ServeConfigBuilder};

    // #[cfg(all(feature = "server", feature = "axum"))]
    // #[cfg_attr(docsrs, doc(cfg(all(feature = "server", feature = "axum"))))]
    // pub use crate::server_context::Axum;

    // #[cfg(feature = "server")]
    // #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
    // pub use crate::server_context::{
    //     extract, server_context, DioxusServerContext, FromContext, FromServerContext,
    //     ProvideServerContext,
    // };

    pub use dioxus_server_macro::*;
    pub use server_fn::{self, ServerFn as _, ServerFnError};
}
