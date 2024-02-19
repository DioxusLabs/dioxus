#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub use once_cell;

mod html_storage;

#[cfg_attr(docsrs, doc(cfg(feature = "axum")))]
#[cfg(feature = "axum")]
mod axum_adapter;

mod config;
mod hooks;
pub mod launch;

#[cfg(all(
    debug_assertions,
    feature = "hot-reload",
    feature = "server",
    not(target_arch = "wasm32")
))]
mod hot_reload;
pub use config::*;

#[cfg(feature = "server")]
mod render;

#[cfg(feature = "server")]
mod serve_config;

#[cfg(feature = "server")]
mod server_context;

/// A prelude of commonly used items in dioxus-fullstack.
pub mod prelude {
    use crate::hooks;
    pub use hooks::{server_cached::server_cached, server_future::use_server_future};

    #[cfg(feature = "axum")]
    #[cfg_attr(docsrs, doc(cfg(feature = "axum")))]
    pub use crate::axum_adapter::*;

    #[cfg(not(feature = "server"))]
    #[cfg_attr(docsrs, doc(cfg(not(feature = "server"))))]
    pub use crate::html_storage::deserialize::get_root_props_from_document;

    #[cfg(all(feature = "server", feature = "router"))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "server", feature = "router"))))]
    pub use crate::render::pre_cache_static_routes_with_props;

    #[cfg(feature = "server")]
    #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
    pub use crate::render::SSRState;

    #[cfg(feature = "router")]
    #[cfg_attr(docsrs, doc(cfg(feature = "router")))]
    pub use crate::router::FullstackRouterConfig;

    #[cfg(feature = "server")]
    #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
    pub use crate::serve_config::{ServeConfig, ServeConfigBuilder};

    #[cfg(all(feature = "server", feature = "axum"))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "server", feature = "axum"))))]
    pub use crate::server_context::Axum;

    #[cfg(feature = "server")]
    #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
    pub use crate::server_context::{
        extract, server_context, DioxusServerContext, FromServerContext, ProvideServerContext,
    };

    #[cfg(feature = "server")]
    #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
    pub use dioxus_ssr::incremental::IncrementalRendererConfig;

    pub use dioxus_server_macro::*;
    pub use server_fn::{self, ServerFn as _, ServerFnError};
}

// // Warn users about overlapping features
// #[cfg(all(feature = "server", feature = "web", not(doc)))]
// compile_error!("The `ssr` feature (enabled by `warp`, `axum`, or `salvo`) and `web` feature are overlapping. Please choose one or the other.");

// #[cfg(all(feature = "server", feature = "desktop", not(doc)))]
// compile_error!("The `ssr` feature (enabled by `warp`, `axum`, or `salvo`) and `desktop` feature are overlapping. Please choose one or the other.");

// #[cfg(all(feature = "server", feature = "mobile", not(doc)))]
// compile_error!("The `ssr` feature (enabled by `warp`, `axum`, or `salvo`) and `mobile` feature are overlapping. Please choose one or the other.");
