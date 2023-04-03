//! # Adapters
//! Adapters for different web frameworks.
//!
//! Each adapter provides a set of utilities that is ergonomic to use with the framework.
//!
//! Each framework has utilies for some or all of the following:
//! - Server functions
//!  - A generic way to register server functions
//!  - A way to register server functions with a custom handler that allows users to pass in a custom [`DioxusServerContext`] based on the state of the server framework.
//! - A way to register static WASM files that is accepts [`ServeConfig`]
//! - A hot reloading web socket that intigrates with [`dioxus-hot-reload`](https://crates.io/crates/dioxus-hot-reload)

#[cfg(feature = "axum")]
pub mod axum_adapter;
#[cfg(feature = "salvo")]
pub mod salvo_adapter;
#[cfg(feature = "warp")]
pub mod warp_adapter;
