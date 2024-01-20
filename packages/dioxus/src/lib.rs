#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

pub use dioxus_core;

#[cfg(feature = "launch")]
mod launch;

#[cfg(feature = "hooks")]
pub use dioxus_hooks as hooks;

#[cfg(feature = "signals")]
pub use dioxus_signals as signals;

pub mod events {
    #[cfg(feature = "html")]
    pub use dioxus_html::prelude::*;
}

#[cfg(feature = "html")]
pub use dioxus_html as html;

#[cfg(feature = "macro")]
pub use dioxus_rsx as rsx;

#[cfg(feature = "macro")]
pub use dioxus_core_macro as core_macro;

pub mod prelude {

    #[cfg(feature = "launch")]
    pub use crate::launch::*;

    #[cfg(feature = "hooks")]
    pub use crate::hooks::*;

    #[cfg(feature = "signals")]
    pub use dioxus_signals::*;

    pub use dioxus_core::prelude::*;

    #[cfg(feature = "macro")]
    #[allow(deprecated)]
    pub use dioxus_core_macro::{component, format_args_f, inline_props, render, rsx, Props};

    #[cfg(feature = "launch")]
    pub use dioxus_config_macro::*;

    #[cfg(feature = "html")]
    pub use dioxus_html as dioxus_elements;

    #[cfg(feature = "html")]
    pub use dioxus_elements::{prelude::*, GlobalAttributes, SvgAttributes};

    #[cfg(all(not(target_arch = "wasm32"), feature = "hot-reload"))]
    pub use dioxus_hot_reload::{self, hot_reload_init};

    pub use dioxus_core;

    #[cfg(feature = "fullstack")]
    pub use dioxus_fullstack::prelude::*;

    #[cfg(feature = "router")]
    pub use dioxus_router;
}

#[cfg(feature = "web")]
pub use dioxus_web as web;

#[cfg(feature = "router")]
pub use dioxus_router as router;

#[cfg(feature = "fullstack")]
pub use dioxus_fullstack as fullstack;

#[cfg(feature = "desktop")]
pub use dioxus_desktop as desktop;

#[cfg(feature = "tui")]
pub use dioxus_tui as tui;

/// Try to evaluate javascript in the target window
///
/// For the browser, this is the window object
/// For desktop/mobile, this is the webview object
///
/// For native, it will try and use the platform's JS engine if available
pub async fn eval(src: String) -> std::result::Result<String, Box<dyn std::error::Error>> {
    todo!()
}
