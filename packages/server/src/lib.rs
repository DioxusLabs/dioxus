#[allow(unused)]
use dioxus_core::prelude::*;

mod adapters;
#[cfg(all(debug_assertions, feature = "hot-reload", feature = "ssr"))]
mod hot_reload;
#[cfg(feature = "ssr")]
pub mod render;
#[cfg(feature = "ssr")]
mod serve;
mod server_context;
mod server_fn;

pub mod prelude {
    #[cfg(feature = "axum")]
    pub use crate::adapters::axum_adapter::*;
    #[cfg(feature = "salvo")]
    pub use crate::adapters::salvo_adapter::*;
    #[cfg(feature = "warp")]
    pub use crate::adapters::warp_adapter::*;
    #[cfg(feature = "ssr")]
    pub use crate::render::*;
    #[cfg(feature = "ssr")]
    pub use crate::serve::{ServeConfig, ServeConfigBuilder};
    pub use crate::server_context::DioxusServerContext;
    pub use crate::server_fn::ServerFn;
    #[cfg(feature = "ssr")]
    pub use crate::server_fn::ServerFnTraitObj;
    pub use server_fn::{self, ServerFn as _, ServerFnError};
    pub use server_macro::*;
}
