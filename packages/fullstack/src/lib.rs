#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![deny(missing_docs)]
#![cfg_attr(any(docsrs, feature = "nightly-doc"), feature(doc_cfg))]

pub use once_cell;

mod html_storage;

#[cfg(feature = "server")]
mod adapters;
// Splitting up the glob export lets us document features required for each adapter
#[cfg_attr(any(docsrs, feature = "nightly-doc"), doc(cfg(feature = "axum")))]
#[cfg(feature = "axum")]
pub use adapters::axum_adapter;
// TODO: Compilation seems to be broken with the salvo feature enabled. Fix and add more features to checks in CI
// #[cfg_attr(any(docsrs, feature = "nightly-doc"), doc(cfg(feature = "salvo")))]
// #[cfg(feature = "salvo")]
// pub use adapters::salvo_adapter;
#[cfg_attr(any(docsrs, feature = "nightly-doc"), doc(cfg(feature = "warp")))]
#[cfg(feature = "warp")]
pub use adapters::warp_adapter;
#[cfg_attr(any(docsrs, feature = "nightly-doc"), doc(cfg(feature = "server")))]
#[cfg(feature = "server")]
pub use adapters::{server_fn_service, ServerFnHandler};
mod config;
mod hooks;
#[cfg(all(debug_assertions, feature = "hot-reload", feature = "server"))]
mod hot_reload;
pub mod launch;
pub use config::*;
#[cfg(feature = "server")]
mod layer;
#[cfg(feature = "server")]
mod render;
#[cfg(feature = "server")]
mod serve_config;
#[cfg(feature = "server")]
mod server_context;
mod server_fn;

/// A prelude of commonly used items in dioxus-fullstack.
pub mod prelude {
    #[cfg(feature = "axum")]
    #[cfg_attr(any(docsrs, feature = "nightly-doc"), doc(cfg(feature = "axum")))]
    pub use crate::adapters::axum_adapter::*;
    // #[cfg(feature = "salvo")]
    // #[cfg_attr(any(docsrs, feature = "nightly-doc"), doc(cfg(feature = "salvo")))]
    // pub use crate::adapters::salvo_adapter::*;
    #[cfg(feature = "warp")]
    #[cfg_attr(any(docsrs, feature = "nightly-doc"), doc(cfg(feature = "warp")))]
    pub use crate::adapters::warp_adapter::*;
    use crate::hooks;
    #[cfg(not(feature = "server"))]
    #[cfg_attr(
        any(docsrs, feature = "nightly-doc"),
        doc(cfg(not(feature = "server")))
    )]
    pub use crate::html_storage::deserialize::get_root_props_from_document;
    #[cfg(feature = "server")]
    #[cfg_attr(any(docsrs, feature = "nightly-doc"), doc(cfg(feature = "server")))]
    pub use crate::layer::{Layer, Service};
    #[cfg(all(feature = "server", feature = "router"))]
    #[cfg_attr(
        any(docsrs, feature = "nightly-doc"),
        doc(cfg(all(feature = "server", feature = "router")))
    )]
    pub use crate::render::pre_cache_static_routes_with_props;
    #[cfg(feature = "server")]
    #[cfg_attr(any(docsrs, feature = "nightly-doc"), doc(cfg(feature = "server")))]
    pub use crate::render::SSRState;
    #[cfg(feature = "router")]
    #[cfg_attr(any(docsrs, feature = "nightly-doc"), doc(cfg(feature = "router")))]
    pub use crate::router::FullstackRouterConfig;
    #[cfg(feature = "server")]
    #[cfg_attr(any(docsrs, feature = "nightly-doc"), doc(cfg(feature = "server")))]
    pub use crate::serve_config::{ServeConfig, ServeConfigBuilder};
    #[cfg(all(feature = "server", feature = "axum"))]
    #[cfg_attr(
        any(docsrs, feature = "nightly-doc"),
        doc(cfg(all(feature = "server", feature = "axum")))
    )]
    pub use crate::server_context::Axum;
    #[cfg(feature = "server")]
    #[cfg_attr(any(docsrs, feature = "nightly-doc"), doc(cfg(feature = "server")))]
    pub use crate::server_context::{
        extract, server_context, DioxusServerContext, FromServerContext, ProvideServerContext,
    };
    pub use crate::server_fn::DioxusServerFn;
    #[cfg(feature = "server")]
    #[cfg_attr(any(docsrs, feature = "nightly-doc"), doc(cfg(feature = "server")))]
    pub use crate::server_fn::{ServerFnMiddleware, ServerFnTraitObj, ServerFunction};
    pub use dioxus_server_macro::*;
    #[cfg(feature = "server")]
    #[cfg_attr(any(docsrs, feature = "nightly-doc"), doc(cfg(feature = "server")))]
    pub use dioxus_ssr::incremental::IncrementalRendererConfig;
    pub use server_fn::{self, ServerFn as _, ServerFnError};

    pub use hooks::{server_cached::server_cached, server_future::use_server_future};
}

// Warn users about overlapping features
#[cfg(all(
    feature = "server",
    feature = "web",
    not(doc),
    not(feature = "nightly-doc")
))]
compile_error!("The `ssr` feature (enabled by `warp`, `axum`, or `salvo`) and `web` feature are overlapping. Please choose one or the other.");

#[cfg(all(
    feature = "server",
    feature = "desktop",
    not(doc),
    not(feature = "nightly-doc")
))]
compile_error!("The `ssr` feature (enabled by `warp`, `axum`, or `salvo`) and `desktop` feature are overlapping. Please choose one or the other.");

#[cfg(all(
    feature = "server",
    feature = "mobile",
    not(doc),
    not(feature = "nightly-doc")
))]
compile_error!("The `ssr` feature (enabled by `warp`, `axum`, or `salvo`) and `mobile` feature are overlapping. Please choose one or the other.");
