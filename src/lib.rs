#![doc = include_str!("../notes/README.md")]

pub use dioxus_core as core;

pub mod hooks {
    #[cfg(feature = "hooks")]
    pub use dioxus_hooks::*;

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    pub use dioxus_web::use_eval;

    #[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
    pub use dioxus_desktop::use_eval;
}

#[cfg(feature = "router")]
pub use dioxus_router as router;

#[cfg(feature = "ssr")]
pub use dioxus_ssr as ssr;

#[cfg(feature = "web")]
pub use dioxus_web as web;

#[cfg(feature = "liveview")]
pub use dioxus_liveview as liveview;

#[cfg(feature = "desktop")]
pub use dioxus_desktop as desktop;

#[cfg(feature = "tui")]
pub use dioxus_tui as tui;

#[cfg(feature = "fermi")]
pub use fermi;

pub mod events {
    #[cfg(feature = "html")]
    pub use dioxus_html::{on::*, KeyCode};
}

pub mod prelude {
    pub use crate::hooks::*;
    pub use dioxus_core::prelude::*;
    pub use dioxus_core_macro::{format_args_f, inline_props, rsx, Props};
    pub use dioxus_elements::{GlobalAttributes, SvgAttributes};
    pub use dioxus_html as dioxus_elements;

    #[cfg(feature = "router")]
    pub use dioxus_router::{use_route, use_router, Link, Redirect, Route, Router, UseRoute};

    #[cfg(feature = "fermi")]
    pub use fermi::{use_atom_ref, use_init_atom_root, use_read, use_set, Atom, AtomRef};
}
