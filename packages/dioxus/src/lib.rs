#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

pub use dioxus_core as core;

#[cfg(feature = "hooks")]
pub use dioxus_hooks as hooks;

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
    #[cfg(feature = "hooks")]
    pub use crate::hooks::*;

    pub use dioxus_core::prelude::*;

    #[cfg(feature = "macro")]
    #[allow(deprecated)]
    pub use dioxus_core_macro::{component, format_args_f, inline_props, render, rsx, Props};

    #[cfg(feature = "html")]
    pub use dioxus_html as dioxus_elements;

    #[cfg(feature = "html")]
    pub use dioxus_elements::{prelude::*, GlobalAttributes, SvgAttributes};

    #[cfg(all(not(target_arch = "wasm32"), feature = "hot-reload"))]
    pub use dioxus_hot_reload::{self, hot_reload_init};
}
