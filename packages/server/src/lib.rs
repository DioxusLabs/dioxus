//! Fullstack utilities for the [`dioxus`](https://dioxuslabs.com) framework.
//!
//! # Features
//! - Intigrations with the [axum](crate::adapters::axum_adapter), [salvo](crate::adapters::salvo_adapters), and [warp](crate::adapters::warp_adapters) server frameworks with utilities for serving and rendering Dioxus applications.
//! - Server functions that allow you to call code on the server from the client as if it were a normal function.
//! - Instant RSX Hot reloading with [`dioxus-hot-reload`](https://crates.io/crates/dioxus-hot-reload).
//!
//! # Example
//! ```rust
//! #![allow(non_snake_case)]
//! use dioxus::prelude::*;
//! use dioxus_server::prelude::*;
//!
//! fn main() {
//!     #[cfg(feature = "web")]
//!     // Hydrate the application on the client
//!     dioxus_web::launch_cfg(app, dioxus_web::Config::new().hydrate(true));
//!     #[cfg(feature = "ssr")]
//!     {
//!         GetServerData::register().unwrap();
//!         tokio::runtime::Runtime::new()
//!             .unwrap()
//!             .block_on(async move {
//!                 let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
//!                 axum::Server::bind(&addr)
//!                     .serve(
//!                         axum::Router::new()
//!                             // Server side render the application, serve static assets, and register server functions
//!                             .serve_dioxus_application("", ServeConfigBuilder::new(app, ()))
//!                             .into_make_service(),
//!                     )
//!                     .await
//!                     .unwrap();
//!             });
//!      }
//! }
//!
//! fn app(cx: Scope) -> Element {
//!     let text = use_state(cx, || "...".to_string());
//!
//!     cx.render(rsx! {
//!         button {
//!             onclick: move |_| {
//!                 to_owned![text];
//!                 async move {
//!                     if let Ok(data) = get_server_data().await {
//!                         text.set(data.clone());
//!                     }
//!                 }
//!             },
//!             "Run a server function"
//!         }
//!         "Server said: {text}"
//!     })
//! }
//!
//! #[server(GetServerData)]
//! async fn get_server_data() -> Result<String, ServerFnError> {
//!     Ok("Hello from the server!".to_string())
//! }
//! ```

#![warn(missing_docs)]
#[allow(unused)]
use dioxus_core::prelude::*;

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
