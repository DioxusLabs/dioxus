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
    pub use dioxus_core_macro::{format_args_f, inline_props, render, rsx, Props};

    #[cfg(feature = "html")]
    pub use dioxus_html as dioxus_elements;

    #[cfg(feature = "html")]
    pub use dioxus_elements::{prelude::*, GlobalAttributes, SvgAttributes};
}
