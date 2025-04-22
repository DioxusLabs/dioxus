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

mod document;
mod launch;
mod render;
mod router;
mod rt;
mod serve_config;
mod server_context;
mod streaming;

pub(crate) use serve_config::*;

pub use crate::render::{FullstackHTMLTemplate, SSRState};
pub use crate::rt::*;
pub use crate::serve_config::{ServeConfig, ServeConfigBuilder};
pub use crate::server_context::Axum;
pub use document::ServerDocument;
pub use launch::launch;
pub use serve_config::*;
pub use server_context::{
    extract, server_context, with_server_context, DioxusServerContext, FromContext,
    FromServerContext, ProvideServerContext,
};
