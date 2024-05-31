#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub use dioxus_core;

#[cfg(feature = "launch")]
#[cfg_attr(docsrs, doc(cfg(feature = "launch")))]
mod launch;

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

pub mod events {
    #[cfg(feature = "html")]
    #[cfg_attr(docsrs, doc(cfg(feature = "html")))]
    pub use dioxus_html::prelude::*;
}

#[cfg(feature = "html")]
#[cfg_attr(docsrs, doc(cfg(feature = "html")))]
pub use dioxus_html as html;

#[cfg(feature = "macro")]
#[cfg_attr(docsrs, doc(cfg(feature = "macro")))]
pub use dioxus_core_macro as core_macro;

pub mod prelude {
    #[cfg(feature = "launch")]
    #[cfg_attr(docsrs, doc(cfg(feature = "launch")))]
    pub use crate::launch::*;

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
    pub use dioxus_core_macro::{component, format_args_f, inline_props, render, rsx, Props};

    #[cfg(feature = "launch")]
    #[cfg_attr(docsrs, doc(cfg(feature = "launch")))]
    pub use dioxus_config_macro::*;

    #[cfg(feature = "html")]
    #[cfg_attr(docsrs, doc(cfg(feature = "html")))]
    pub use dioxus_html as dioxus_elements;

    #[cfg(feature = "html")]
    #[cfg_attr(docsrs, doc(cfg(feature = "html")))]
    pub use dioxus_elements::{global_attributes, prelude::*, svg_attributes};

    #[cfg(all(
        not(any(target_arch = "wasm32", target_os = "ios", target_os = "android")),
        feature = "hot-reload"
    ))]
    #[cfg_attr(docsrs, doc(cfg(feature = "hot-reload")))]
    pub use dioxus_hot_reload::{self, hot_reload_init};

    pub use dioxus_core;

    #[cfg(feature = "fullstack")]
    #[cfg_attr(docsrs, doc(cfg(feature = "fullstack")))]
    pub use dioxus_fullstack::prelude::*;

    #[cfg(feature = "router")]
    #[cfg_attr(docsrs, doc(cfg(feature = "router")))]
    pub use dioxus_router;

    #[cfg(feature = "router")]
    #[cfg_attr(docsrs, doc(cfg(feature = "router")))]
    pub use dioxus_router::prelude::*;
}

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
