//! Dioxus utilities for the [Axum](https://docs.rs/axum/latest/axum/index.html) server framework.
//!
//! # Example
//! ```rust, no_run
//! #![allow(non_snake_case)]
//! use dioxus::prelude::*;
//!
//! fn main() {
//!     #[cfg(feature = "web")]
//!     // Hydrate the application on the client
//!     dioxus::launch(app);
//!     #[cfg(feature = "server")]
//!     {
//!         tokio::runtime::Runtime::new()
//!             .unwrap()
//!             .block_on(async move {
//!                 // Get the address the server should run on. If the CLI is running, the CLI proxies fullstack into the main address
//!                 // and we use the generated address the CLI gives us
//!                 let address = dioxus::cli_config::fullstack_address_or_localhost();
//!                 let listener = tokio::net::TcpListener::bind(address)
//!                     .await
//!                     .unwrap();
//!                 axum::serve(
//!                         listener,
//!                         axum::Router::new()
//!                             // Server side render the application, serve static assets, and register server functions
//!                             .serve_dioxus_application(ServeConfigBuilder::default(), app)
//!                             .into_make_service(),
//!                     )
//!                     .await
//!                     .unwrap();
//!             });
//!      }
//! }
//!
//! fn app() -> Element {
//!
//!
//!     rsx! {
//!         button {
//!             onclick: move |_| async move {
//!                 if let Ok(data) = get_server_data().await {
//!                     text.set(data);
//!                 }
//!             },
//!             "Run a server function"
//!         }
//!         "Server said: {text}"
//!     }
//! }
//!
//! #[server(GetServerData)]
//! async fn get_server_data() -> Result<String, ServerFnError> {
//!     Ok("Hello from the server!".to_string())
//! }
//! ```

pub mod config;
pub mod context;

mod document;
mod render;
mod server;
mod streaming;

pub(crate) use config::*;

pub use crate::config::{ServeConfig, ServeConfigBuilder};
pub use crate::context::Axum;
pub use crate::render::{FullstackHTMLTemplate, SSRState};
pub use crate::server::*;
pub use config::*;
pub use context::{
    extract, server_context, with_server_context, DioxusServerContext, FromContext,
    FromServerContext, ProvideServerContext,
};
pub use document::ServerDocument;

#[cfg(not(target_arch = "wasm32"))]
mod launch;

#[cfg(not(target_arch = "wasm32"))]
pub use launch::{launch, launch_cfg};

/// Re-export commonly used items
pub mod prelude {
    pub use crate::config::{ServeConfig, ServeConfigBuilder};
    pub use crate::context::Axum;
    pub use crate::context::{
        extract, server_context, with_server_context, DioxusServerContext, FromContext,
        FromServerContext, ProvideServerContext,
    };
    pub use crate::render::{FullstackHTMLTemplate, SSRState};
    pub use crate::server::*;
    pub use dioxus_isrg::{IncrementalRenderer, IncrementalRendererConfig};
}

pub use dioxus_isrg::{IncrementalRenderer, IncrementalRendererConfig};
