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

pub(crate) use document::*;
pub(crate) use launch::*;
pub(crate) use render::*;
pub(crate) use router::*;
pub(crate) use rt::*;
pub(crate) use serve_config::*;
pub(crate) use server_context::*;
pub(crate) use streaming::*;

pub use launch::launch;

// #[cfg(feature = "server")]
// pub mod server;

// #[cfg(feature = "server")]
// pub use server::ServerDocument;

// #[cfg(feature = "axum")]
// #[cfg_attr(docsrs, doc(cfg(feature = "axum")))]
// pub mod server;

// #[cfg(feature = "axum_core")]
// #[cfg_attr(docsrs, doc(cfg(feature = "axum_core")))]
// pub mod axum_core;

// pub mod document;
// #[cfg(feature = "server")]
// mod render;
// #[cfg(feature = "server")]
// mod streaming;

// #[cfg(feature = "server")]
// mod serve_config;

// #[cfg(feature = "server")]
// pub use serve_config::*;

// #[cfg(feature = "server")]
// mod server_context;

// #[cfg(feature = "axum")]
// #[cfg_attr(docsrs, doc(cfg(feature = "axum")))]
// pub use crate::server::*;

// #[cfg(feature = "axum_core")]
// pub use crate::axum_core::*;

// #[cfg(feature = "server")]
// #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
// pub use crate::render::{FullstackHTMLTemplate, SSRState};

// #[cfg(feature = "server")]
// #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
// pub use crate::serve_config::{ServeConfig, ServeConfigBuilder};

// #[cfg(feature = "axum")]
// #[cfg_attr(docsrs, doc(cfg(feature = "axum")))]
// pub use crate::server_context::Axum;

// #[cfg(feature = "server")]
// #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
// pub use crate::server_context::{
//     extract, server_context, with_server_context, DioxusServerContext, FromContext,
//     FromServerContext, ProvideServerContext,
// };
