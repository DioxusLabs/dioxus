#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

pub use dioxus_core as core;

#[cfg(feature = "hooks")]
pub use dioxus_hooks as hooks;

#[cfg(feature = "signals")]
pub use dioxus_signals as signals;

pub mod events {
    // #[cfg(feature = "html")]
    // pub use dioxus_html::prelude::*;
}

#[cfg(feature = "html")]
pub use dioxus_html as html;

#[cfg(feature = "html")]
pub use dioxus_document as document;

#[cfg(feature = "html")]
pub use dioxus_history as history;

#[cfg(feature = "macro")]
pub use dioxus_rsx as rsx;

#[cfg(feature = "macro")]
pub use dioxus_core_macro as core_macro;

#[cfg(feature = "warnings")]
pub use warnings;

pub mod prelude {
    #[cfg(feature = "html")]
    #[cfg_attr(docsrs, doc(cfg(feature = "html")))]
    pub use dioxus_history::{history, History};

    #[cfg(feature = "hooks")]
    pub use crate::hooks::*;

    #[cfg(feature = "signals")]
    pub use dioxus_signals::*;

    #[cfg(feature = "macro")]
    #[allow(deprecated)]
    pub use dioxus_core_macro::{component, rsx, Props};

    #[cfg(feature = "macro")]
    pub use dioxus_config_macro::*;

    #[cfg(feature = "html")]
    pub use dioxus_html as dioxus_elements;

    #[cfg(feature = "html")]
    pub use dioxus_elements::{events::*, global_attributes, svg_attributes, traits::*};

    pub use dioxus_core;

    pub use dioxus_core::{
        consume_context, provide_context, spawn, suspend, try_consume_context, use_hook, Attribute,
        Callback, Component, Context, Element, ErrorBoundary, ErrorContext, Event, EventHandler,
        Fragment, HasAttributes, IntoDynNode, RenderError, ScopeId, SuspenseBoundary,
        SuspenseContext, SuspenseExtension, VNode, VirtualDom,
    };
}
