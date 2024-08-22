#![doc = include_str!("../README.md")]
//!
//! ## Dioxus Crate Features
//!
//! This crate has several features that can be enabled to change the active renderer and enable various integrations:
//!
//! - `signals`: (default) re-exports `dioxus-signals`
//! - `macro`: (default) re-exports `dioxus-macro`
//! - `html`: (default) exports `dioxus-html` as the default elements to use in rsx
//! - `hooks`: (default) re-exports `dioxus-hooks`
//! - `hot-reload`: (default) enables hot rsx reloading in all renderers that support it
//! - `router`: exports the [router](https://dioxuslabs.com/learn/0.5/router) and enables any router features for the current platform
//! - `third-party-renderer`: Just disables warnings about no active platform when no renderers are enabled
//!
//! Platform features (the current platform determines what platform the [`launch()`] function runs):
//!
//! - `fullstack`: enables the fullstack platform. This must be used in combination with the `web` feature for wasm builds and `axum` feature for server builds
//! - `desktop`: enables the desktop platform
//! - `mobile`: enables the mobile platform
//! - `web`: enables the web platform. If the fullstack platform is enabled, this will set the fullstack platform to client mode
//! - `liveview`: enables the liveview platform
//! - `static-generation`: enables the static generation platform. This must be used in combination with the `web` feature for wasm builds and `axum` feature for server builds
//! - `axum`: enables the axum server with static generation or fullstack and sets the platform to server mode
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub use dioxus_core;
pub use dioxus_core::{CapturedError, Ok, Result};

#[cfg(feature = "launch")]
#[cfg_attr(docsrs, doc(cfg(feature = "launch")))]
mod launch;

pub mod events {
    #[cfg(feature = "html")]
    #[cfg_attr(docsrs, doc(cfg(feature = "html")))]
    pub use dioxus_html::prelude::*;
}

#[cfg(feature = "launch")]
#[cfg_attr(docsrs, doc(cfg(feature = "launch")))]
#[allow(deprecated)]
pub use launch::launch;

#[cfg(feature = "hooks")]
#[cfg_attr(docsrs, doc(cfg(feature = "hooks")))]
pub use dioxus_hooks as hooks;

#[cfg(feature = "signals")]
#[cfg_attr(docsrs, doc(cfg(feature = "signals")))]
pub use dioxus_signals as signals;

#[cfg(feature = "html")]
#[cfg_attr(docsrs, doc(cfg(feature = "html")))]
pub use dioxus_html as html;

#[cfg(feature = "macro")]
#[cfg_attr(docsrs, doc(cfg(feature = "macro")))]
pub use dioxus_core_macro as core_macro;

#[cfg(feature = "web")]
#[cfg_attr(docsrs, doc(cfg(feature = "web")))]
pub use dioxus_web as web;

#[cfg(feature = "router")]
#[cfg_attr(docsrs, doc(cfg(feature = "router")))]
pub use dioxus_router as router;

#[cfg(feature = "fullstack")]
#[cfg_attr(docsrs, doc(cfg(feature = "fullstack")))]
pub use dioxus_fullstack as fullstack;

#[cfg(feature = "static-generation")]
#[cfg_attr(docsrs, doc(cfg(feature = "static-generation")))]
pub use dioxus_static_site_generation as static_site_generation;

#[cfg(feature = "desktop")]
#[cfg_attr(docsrs, doc(cfg(feature = "desktop")))]
pub use dioxus_desktop as desktop;

#[cfg(feature = "mobile")]
#[cfg_attr(docsrs, doc(cfg(feature = "mobile")))]
pub use dioxus_mobile as mobile;

#[cfg(feature = "liveview")]
#[cfg_attr(docsrs, doc(cfg(feature = "liveview")))]
pub use dioxus_liveview as liveview;

#[cfg(feature = "ssr")]
#[cfg_attr(docsrs, doc(cfg(feature = "ssr")))]
pub use dioxus_ssr as ssr;

#[cfg(feature = "asset")]
#[cfg_attr(docsrs, doc(cfg(feature = "asset")))]
pub use manganis as assets;

#[cfg(feature = "asset")]
#[cfg_attr(docsrs, doc(cfg(feature = "asset")))]
pub use manganis;

pub mod prelude {
    #[cfg(feature = "launch")]
    #[cfg_attr(docsrs, doc(cfg(feature = "launch")))]
    pub use crate::launch::*;

    #[cfg(feature = "launch")]
    #[cfg_attr(docsrs, doc(cfg(feature = "launch")))]
    pub use dioxus_config_macro::*;

    #[cfg(feature = "hooks")]
    #[cfg_attr(docsrs, doc(cfg(feature = "hooks")))]
    pub use crate::hooks::*;

    #[cfg(feature = "signals")]
    #[cfg_attr(docsrs, doc(cfg(feature = "signals")))]
    pub use dioxus_signals::*;

    pub use dioxus_core::prelude::*;

    #[cfg(feature = "macro")]
    #[cfg_attr(docsrs, doc(cfg(feature = "macro")))]
    #[allow(deprecated)]
    pub use dioxus_core_macro::{component, rsx, Props};

    #[cfg(feature = "html")]
    #[cfg_attr(docsrs, doc(cfg(feature = "html")))]
    pub use dioxus_html as dioxus_elements;

    #[cfg(feature = "html")]
    #[cfg_attr(docsrs, doc(cfg(feature = "html")))]
    pub use dioxus_elements::{global_attributes, prelude::*, svg_attributes};

    pub use dioxus_core;

    #[cfg(feature = "fullstack")]
    #[cfg_attr(docsrs, doc(cfg(feature = "fullstack")))]
    pub use dioxus_fullstack::prelude::*;

    #[cfg(all(feature = "static-generation", not(feature = "fullstack")))]
    #[cfg_attr(
        docsrs,
        doc(cfg(all(feature = "static-generation", not(feature = "fullstack"))))
    )]
    pub use dioxus_static_site_generation::prelude::*;

    #[cfg(feature = "router")]
    #[cfg_attr(docsrs, doc(cfg(feature = "router")))]
    pub use dioxus_router;

    #[cfg(feature = "router")]
    #[cfg_attr(docsrs, doc(cfg(feature = "router")))]
    pub use dioxus_router::prelude::*;

    #[cfg(feature = "axum")]
    #[cfg_attr(docsrs, doc(cfg(feature = "axum")))]
    pub use axum;

    #[cfg(feature = "asset")]
    #[cfg_attr(docsrs, doc(cfg(feature = "asset")))]
    pub use manganis::{self, self as assets, asset, Asset, ImageAsset, ImageType};

    #[cfg(feature = "document")]
    #[cfg_attr(docsrs, doc(cfg(feature = "document")))]
    pub use dioxus_document as document;
}
