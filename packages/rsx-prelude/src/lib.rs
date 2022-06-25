//! This package is meant for internal use within dioxus. It provides a prelude that enables basic components to work.

pub use dioxus_core as core;

pub mod hooks {
    pub use dioxus_hooks::*;
}

pub use hooks::*;

pub mod events {
    pub use dioxus_html::{on::*, KeyCode};
}

#[cfg(feature = "hot_reload")]
pub use dioxus_rsx_interpreter as rsx_interpreter;

pub mod prelude {
    pub use crate::hooks::*;
    pub use dioxus_core::prelude::*;
    pub use dioxus_core_macro::{format_args_f, inline_props, rsx, Props};
    pub use dioxus_elements::{GlobalAttributes, SvgAttributes};
    pub use dioxus_html as dioxus_elements;

    #[cfg(feature = "hot_reload")]
    pub use dioxus_rsx_interpreter::{
        captuered_context::{CapturedContext, FormattedArg, IfmtArgs},
        get_line_num, resolve_scope, CodeLocation, RsxContext,
    };
}
