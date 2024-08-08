#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub use once_cell;

mod html_storage;

#[cfg(feature = "axum")]
#[cfg_attr(docsrs, doc(cfg(feature = "axum")))]
mod axum_adapter;

mod config;
mod hooks;
pub mod launch;

pub use config::*;

pub mod document;
#[cfg(feature = "server")]
mod render;
#[cfg(feature = "server")]
mod streaming;

#[cfg(feature = "server")]
mod serve_config;
#[cfg(feature = "server")]
pub use serve_config::*;

#[cfg(feature = "server")]
mod server_context;

/// A prelude of commonly used items in dioxus-fullstack.
pub mod prelude {
    use crate::hooks;
    pub use hooks::{server_cached::use_server_cached, server_future::use_server_future};

    #[cfg(feature = "axum")]
    #[cfg_attr(docsrs, doc(cfg(feature = "axum")))]
    pub use crate::axum_adapter::*;

    #[cfg(feature = "server")]
    #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
    pub use crate::render::{FullstackHTMLTemplate, SSRState};

    #[cfg(feature = "server")]
    #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
    pub use crate::serve_config::{ServeConfig, ServeConfigBuilder};

    #[cfg(all(feature = "server", feature = "axum"))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "server", feature = "axum"))))]
    pub use crate::server_context::Axum;

    #[cfg(feature = "server")]
    #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
    pub use crate::server_context::{
        extract, server_context, with_server_context, DioxusServerContext, FromContext,
        FromServerContext, ProvideServerContext,
    };

    #[cfg(feature = "server")]
    #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
    pub use dioxus_ssr::incremental::{IncrementalRenderer, IncrementalRendererConfig};

    pub use dioxus_server_macro::*;
    pub use server_fn::{self, ServerFn as _, ServerFnError};
}
