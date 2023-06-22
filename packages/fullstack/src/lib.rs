#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![deny(missing_docs)]

pub use adapters::*;

mod props_html;

mod adapters;
#[cfg(all(debug_assertions, feature = "hot-reload", feature = "ssr"))]
mod hot_reload;
#[cfg(feature = "router")]
mod incremental;
#[cfg(feature = "ssr")]
mod render;
#[cfg(feature = "ssr")]
mod serve_config;
mod server_context;
mod server_fn;

/// A prelude of commonly used items in dioxus-fullstack.
pub mod prelude {
    #[cfg(feature = "axum")]
    pub use crate::adapters::axum_adapter::*;
    #[cfg(feature = "salvo")]
    pub use crate::adapters::salvo_adapter::*;
    #[cfg(feature = "warp")]
    pub use crate::adapters::warp_adapter::*;
    #[cfg(not(feature = "ssr"))]
    pub use crate::props_html::deserialize_props::get_root_props_from_document;
    #[cfg(feature = "ssr")]
    pub use crate::render::SSRState;
    #[cfg(feature = "ssr")]
    pub use crate::serve_config::{ServeConfig, ServeConfigBuilder};
    #[cfg(feature = "ssr")]
    pub use crate::server_context::RequestParts;
    pub use crate::server_context::{DioxusServerContext, HasServerContext};
    pub use crate::server_fn::DioxusServerFn;
    #[cfg(feature = "ssr")]
    pub use crate::server_fn::{ServerFnTraitObj, ServerFunction};
    pub use dioxus_server_macro::*;
    pub use server_fn::{self, ServerFn as _, ServerFnError};
}
