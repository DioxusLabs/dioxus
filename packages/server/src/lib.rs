//! Fullstack utilities for the [`dioxus`](https://dioxuslabs.com) framework.
//!
//! # Features
//! - Intigrations with the [axum](crate::adapters::axum_adapter), [salvo](crate::adapters::salvo_adapter), and [warp](crate::adapters::warp_adapter) server frameworks with utilities for serving and rendering Dioxus applications.
//! - [Server functions](crate::prelude::server) that allow you to call code on the server from the client as if it were a normal function.
//! - Instant RSX Hot reloading with [`dioxus-hot-reload`](https://crates.io/crates/dioxus-hot-reload).
//!
//! # Example
//! Full stack Dioxus in under 50 lines of code
//! ```rust
//! #![allow(non_snake_case)]
//! use dioxus::prelude::*;
//! use dioxus_server::prelude::*;
//!
//! fn main() {
//!     #[cfg(feature = "web")]
//!     dioxus_web::launch_cfg(app, dioxus_web::Config::new().hydrate(true));
//!     #[cfg(feature = "ssr")]
//!     {
//!         GetMeaning::register().unwrap();
//!         tokio::runtime::Runtime::new()
//!             .unwrap()
//!             .block_on(async move {
//!                 warp::serve(serve_dioxus_application(
//!                     "",
//!                     ServeConfigBuilder::new(app, ()),
//!                 ))
//!                 .run(([127, 0, 0, 1], 8080))
//!                 .await;
//!             });
//!     }
//! }
//!
//! fn app(cx: Scope) -> Element {
//!     let meaning = use_state(cx, || None);
//!     cx.render(rsx! {
//!         button {
//!             onclick: move |_| {
//!                 to_owned![meaning];
//!                 async move {
//!                     if let Ok(data) = get_meaning("life the universe and everything".into()).await {
//!                         meaning.set(data);
//!                     }
//!                 }
//!             },
//!             "Run a server function"
//!         }
//!         "Server said: {meaning:?}"
//!     })
//! }
//!
//! // This code will only run on the server
//! #[server(GetMeaning)]
//! async fn get_meaning(of: String) -> Result<Option<u32>, ServerFnError> {
//!     Ok(of.contains("life").then(|| 42))
//! }
//! ```

#![warn(missing_docs)]
#[allow(unused)]
use dioxus_core::prelude::*;

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
    pub use crate::serve_config::{ServeConfig, ServeConfigBuilder};
    pub use crate::server_context::DioxusServerContext;
    pub use crate::server_fn::ServerFn;
    #[cfg(feature = "ssr")]
    pub use crate::server_fn::ServerFnTraitObj;
    pub use server_fn::{self, ServerFn as _, ServerFnError};
    pub use server_macro::*;
}
