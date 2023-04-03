#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![deny(missing_docs)]

pub use adapters::*;

mod adapters;
#[cfg(all(debug_assertions, feature = "hot-reload", feature = "ssr"))]
mod hot_reload;
#[cfg(feature = "ssr")]
mod render;
#[cfg(feature = "ssr")]
mod serve_config;
mod server_context;
mod server_fn;

/// A prelude of commonly used items in dioxus-server.
pub mod prelude {
    #[cfg(feature = "axum")]
    pub use crate::adapters::axum_adapter::*;
    #[cfg(feature = "salvo")]
    pub use crate::adapters::salvo_adapter::*;
    #[cfg(feature = "warp")]
    pub use crate::adapters::warp_adapter::*;
    #[cfg(feature = "ssr")]
    pub use crate::render::SSRState;
    #[cfg(feature = "ssr")]
    pub use crate::serve_config::{ServeConfig, ServeConfigBuilder};
    pub use crate::server_context::DioxusServerContext;
    pub use crate::server_fn::ServerFn;
    #[cfg(feature = "ssr")]
    pub use crate::server_fn::ServerFnTraitObj;
    pub use dioxus_server_macro::*;
    pub use server_fn::{self, ServerFn as _, ServerFnError};
}
