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
//! - `router`: exports the [router](https://dioxuslabs.com/learn/0.7/essentials/router/) and enables any router features for the current platform
//! - `third-party-renderer`: Just disables warnings about no active platform when no renderers are enabled
//! - `logger`: Enable the default tracing subscriber for Dioxus apps
//!
//! Platform features (the current platform determines what platform the [`launch()`] function runs):
//!
//! - `fullstack`: enables the fullstack platform. This must be used in combination with the `web` feature for wasm builds and `server` feature for server builds
//! - `desktop`: enables the desktop platform
//! - `mobile`: enables the mobile platform
//! - `web`: enables the web platform. If the fullstack platform is enabled, this will set the fullstack platform to client mode
//! - `liveview`: enables the liveview platform
//! - `server`: enables the server variant of dioxus
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub use dioxus_core;
#[doc(inline)]
pub use dioxus_core::{CapturedError, Ok, Result};

#[cfg(feature = "launch")]
#[cfg_attr(docsrs, doc(cfg(feature = "launch")))]
mod launch;

pub use dioxus_core as core;

#[cfg(feature = "launch")]
#[cfg_attr(docsrs, doc(cfg(feature = "launch")))]
pub use crate::launch::*;

#[cfg(feature = "hooks")]
#[cfg_attr(docsrs, doc(cfg(feature = "hooks")))]
pub use dioxus_hooks as hooks;

#[cfg(feature = "signals")]
#[cfg_attr(docsrs, doc(cfg(feature = "signals")))]
pub use dioxus_signals as signals;

#[cfg(feature = "signals")]
#[cfg_attr(docsrs, doc(cfg(feature = "signals")))]
pub use dioxus_stores as stores;

pub mod events {
    #[cfg(feature = "html")]
    #[cfg_attr(docsrs, doc(cfg(feature = "html")))]
    pub use dioxus_html::events::*;
}

#[cfg(feature = "document")]
#[cfg_attr(docsrs, doc(cfg(feature = "document")))]
pub use dioxus_document as document;

#[cfg(feature = "document")]
#[cfg_attr(docsrs, doc(cfg(feature = "document")))]
pub use dioxus_history as history;

#[cfg(feature = "html")]
#[cfg_attr(docsrs, doc(cfg(feature = "html")))]
pub use dioxus_html as html;

#[cfg(feature = "macro")]
#[cfg_attr(docsrs, doc(cfg(feature = "macro")))]
pub use dioxus_core_macro as core_macro;

#[cfg(feature = "logger")]
#[cfg_attr(docsrs, doc(cfg(feature = "logger")))]
pub use dioxus_logger as logger;

#[cfg(feature = "cli-config")]
#[cfg_attr(docsrs, doc(cfg(feature = "cli-config")))]
pub use dioxus_cli_config as cli_config;

#[cfg(feature = "server")]
#[cfg_attr(docsrs, doc(cfg(feature = "server")))]
pub use dioxus_server as server;

#[cfg(feature = "server")]
pub use dioxus_server::serve;

#[cfg(feature = "devtools")]
#[cfg_attr(docsrs, doc(cfg(feature = "devtools")))]
pub use dioxus_devtools as devtools;

#[cfg(feature = "web")]
#[cfg_attr(docsrs, doc(cfg(feature = "web")))]
pub use dioxus_web as web;

#[cfg(feature = "router")]
#[cfg_attr(docsrs, doc(cfg(feature = "router")))]
pub use dioxus_router as router;

#[cfg(feature = "fullstack")]
#[cfg_attr(docsrs, doc(cfg(feature = "fullstack")))]
pub use dioxus_fullstack as fullstack;

#[cfg(feature = "desktop")]
#[cfg_attr(docsrs, doc(cfg(feature = "desktop")))]
pub use dioxus_desktop as desktop;

#[cfg(feature = "mobile")]
#[cfg_attr(docsrs, doc(cfg(feature = "mobile")))]
pub use dioxus_desktop as mobile;

#[cfg(feature = "liveview")]
#[cfg_attr(docsrs, doc(cfg(feature = "liveview")))]
pub use dioxus_liveview as liveview;

#[cfg(feature = "ssr")]
#[cfg_attr(docsrs, doc(cfg(feature = "ssr")))]
pub use dioxus_ssr as ssr;

#[cfg(feature = "warnings")]
#[cfg_attr(docsrs, doc(cfg(feature = "warnings")))]
pub use warnings;

pub use dioxus_config_macros as config_macros;

#[cfg(feature = "wasm-split")]
#[cfg_attr(docsrs, doc(cfg(feature = "wasm-split")))]
pub use wasm_splitter as wasm_split;

pub use subsecond;

#[cfg(feature = "asset")]
#[cfg_attr(docsrs, doc(cfg(feature = "asset")))]
#[doc(inline)]
pub use dioxus_asset_resolver as asset_resolver;

pub mod prelude {
    #[cfg(feature = "document")]
    #[cfg_attr(docsrs, doc(cfg(feature = "document")))]
    #[doc(inline)]
    pub use dioxus_document::{self as document, Meta, Stylesheet, Title};

    #[cfg(feature = "document")]
    #[cfg_attr(docsrs, doc(cfg(feature = "document")))]
    #[doc(inline)]
    pub use dioxus_document::builder::*;

    #[cfg(feature = "document")]
    #[cfg_attr(docsrs, doc(cfg(feature = "document")))]
    #[doc(inline)]
    pub use dioxus_history::{history, History};

    #[cfg(feature = "launch")]
    #[cfg_attr(docsrs, doc(cfg(feature = "launch")))]
    #[doc(inline)]
    pub use crate::launch::*;

    #[cfg(feature = "hooks")]
    #[cfg_attr(docsrs, doc(cfg(feature = "hooks")))]
    #[doc(inline)]
    pub use crate::hooks::*;

    #[cfg(feature = "signals")]
    #[cfg_attr(docsrs, doc(cfg(feature = "signals")))]
    #[doc(inline)]
    pub use dioxus_signals::{self, *};

    #[cfg(feature = "signals")]
    pub use dioxus_stores::{self, store, use_store, GlobalStore, ReadStore, Store, WriteStore};

    #[cfg(feature = "macro")]
    #[cfg_attr(docsrs, doc(cfg(feature = "macro")))]
    #[allow(deprecated)]
    #[doc(inline)]
    pub use dioxus_core_macro::{component, rsx, Props};

    #[cfg(feature = "launch")]
    #[cfg_attr(docsrs, doc(cfg(feature = "launch")))]
    pub use dioxus_config_macro::*;

    #[cfg(feature = "html")]
    #[cfg_attr(docsrs, doc(cfg(feature = "html")))]
    pub use dioxus_html as dioxus_elements;

    #[cfg(feature = "html")]
    #[cfg_attr(docsrs, doc(cfg(feature = "html")))]
    #[doc(inline)]
    pub use dioxus_elements::{Code, Key, Location, Modifiers};

    #[cfg(feature = "html")]
    #[cfg_attr(docsrs, doc(cfg(feature = "html")))]
    #[doc(no_inline)]
    pub use dioxus_elements::{
        events::*, extensions::*, global_attributes, keyboard_types, svg_attributes, traits::*,
        GlobalAttributesExtension, SvgAttributesExtension,
    };

    #[cfg(feature = "html")]
    #[cfg_attr(docsrs, doc(cfg(feature = "html")))]
    #[doc(inline)]
    pub use dioxus_elements::builder as builder;

    #[cfg(feature = "html")]
    #[cfg_attr(docsrs, doc(cfg(feature = "html")))]
    #[doc(inline)]
    pub use dioxus_elements::builder::prelude::*;

    #[cfg(feature = "html")]
    #[cfg_attr(docsrs, doc(cfg(feature = "html")))]
    #[doc(inline)]
    pub use dioxus_elements::{data, static_str};

    #[cfg(feature = "devtools")]
    #[cfg_attr(docsrs, doc(cfg(feature = "devtools")))]
    pub use dioxus_devtools;

    pub use dioxus_core;

    #[cfg(feature = "fullstack")]
    #[cfg_attr(docsrs, doc(cfg(feature = "fullstack")))]
    #[doc(inline)]
    pub use dioxus_fullstack::{
        self as dioxus_fullstack, delete, get, patch, post, put, server, use_loader,
        use_server_cached, use_server_future, HttpError, OrHttpError, ServerFnError,
        ServerFnResult, StatusCode,
    };

    #[cfg(feature = "server")]
    #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
    #[doc(inline)]
    pub use dioxus_server::{self, serve, DioxusRouterExt, ServeConfig, ServerFunction};

    #[cfg(feature = "router")]
    #[cfg_attr(docsrs, doc(cfg(feature = "router")))]
    pub use dioxus_router;

    #[cfg(feature = "router")]
    #[cfg_attr(docsrs, doc(cfg(feature = "router")))]
    #[doc(inline)]
    pub use dioxus_router::{
        hooks::*, navigator, use_navigator, GoBackButton, GoForwardButton, Link, NavigationTarget,
        Outlet, Routable, Router,
    };

    #[cfg(feature = "asset")]
    #[cfg_attr(docsrs, doc(cfg(feature = "asset")))]
    #[doc(inline)]
    pub use manganis::{self, *};

    #[cfg(feature = "wasm-split")]
    #[cfg_attr(docsrs, doc(cfg(feature = "wasm-split")))]
    pub use wasm_splitter as wasm_split;

    #[doc(inline)]
    pub use dioxus_core::{
        consume_context, provide_context, spawn, suspend, try_consume_context, use_drop, use_hook,
        AnyhowContext, Attribute, Callback, Component, Element, ErrorBoundary, ErrorContext, Event,
        EventHandler, Fragment, HasAttributes, IntoDynNode, RenderError, Result, ScopeId,
        SuspenseBoundary, SuspenseContext, VNode, VirtualDom,
    };

    #[cfg(feature = "logger")]
    pub use dioxus_logger::tracing::{debug, error, info, trace, warn};
}
