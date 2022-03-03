#![doc = include_str!("../notes/README.md")]

pub use dioxus_core as core;

#[cfg(feature = "hooks")]
pub use dioxus_hooks as hooks;

#[cfg(feature = "router")]
pub use dioxus_router as router;

#[cfg(feature = "ssr")]
pub use dioxus_ssr as ssr;

#[cfg(feature = "web")]
pub use dioxus_web as web;

#[cfg(feature = "desktop")]
pub use dioxus_desktop as desktop;

#[cfg(feature = "fermi")]
pub use fermi;

// #[cfg(feature = "mobile")]
// pub use dioxus_mobile as mobile;

pub mod events {
    #[cfg(feature = "html")]
    pub use dioxus_html::{on::*, KeyCode};
}

pub mod prelude {
    pub use dioxus_core::prelude::*;
    pub use dioxus_core_macro::{format_args_f, inline_props, rsx, Props};
    pub use dioxus_elements::{GlobalAttributes, SvgAttributes};
    pub use dioxus_hooks::*;
    pub use dioxus_html as dioxus_elements;
}
