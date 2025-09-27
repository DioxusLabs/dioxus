#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
// #![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unexpected_cfgs)]

// re-exported to make it possible to implement a custom Client without adding a separate
// dependency on `bytes`
pub use bytes::Bytes;
pub use dioxus_fullstack_core::client::{get_server_url, set_server_url};
pub use dioxus_fullstack_core::{ServerFnError, ServerFnResult};

pub(crate) use config::*;
pub use config::*;
pub use config::{ServeConfig, ServeConfigBuilder};
pub use context::Axum;
pub use context::{
    extract, server_context, with_server_context, DioxusServerContext, FromContext,
    FromServerContext, ProvideServerContext,
};
pub use dioxus_isrg::{IncrementalRenderer, IncrementalRendererConfig};
pub use document::ServerDocument;
pub use server::*;

pub use axum;
#[doc(hidden)]
pub use const_format;
#[doc(hidden)]
pub use const_str;
pub use http;
#[doc(hidden)]
pub use inventory;
#[doc(hidden)]
pub use serde;
#[doc(hidden)]
pub use xxhash_rust;

pub mod redirect;

/// Implementations of the client side of the server function call.
pub mod client;
pub use client::*;

// #![deny(missing)]

// #[doc(hidden)]
// #[cfg(feature = "serde-lite")]
// pub use serde_lite;

// #[cfg(feature = "axum-no-default")]
// #[doc(hidden)]
// pub use ::axum as axum_export;

// #[cfg(feature = "generic")]
// #[doc(hidden)]
// pub use ::bytes as bytes_export;
// #[cfg(feature = "generic")]
// #[doc(hidden)]
// pub use ::http as http_export;
// #[cfg(feature = "rkyv")]
// pub use rkyv;

// pub mod server_fn {
//     // pub use crate::{
//     //     client,
//     //     client::{get_server_url, set_server_url},
//     //     codec, server, BoxedStream, ContentType, Decodes, Encodes, Format, FormatType, ServerFn,
//     //     Websocket,
//     // };
//     pub use serde;
// }

#[cfg(not(target_arch = "wasm32"))]
mod launch;

#[cfg(not(target_arch = "wasm32"))]
pub use launch::{launch, launch_cfg};

/// Implementations of the server side of the server function call.
pub mod server;

/// Types and traits for HTTP responses.
// pub mod response;
pub mod config;
pub mod context;

pub(crate) mod document;
pub(crate) mod ssr;

pub mod serverfn;
pub use serverfn::*;

pub mod prelude {}

pub mod streaming;
pub use streaming::*;

pub use launch::router;
pub use launch::serve;
