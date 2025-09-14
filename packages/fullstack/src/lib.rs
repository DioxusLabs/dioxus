#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
// #![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unexpected_cfgs)]

// re-exported to make it possible to implement a custom Client without adding a separate
// dependency on `bytes`
pub use bytes::Bytes;
pub use client::{get_server_url, set_server_url};
pub use error::{ServerFnError, ServerFnResult};

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

// #![deny(missing)]

// #[doc(hidden)]
// #[cfg(feature = "serde-lite")]
// pub use serde_lite;

#[cfg(feature = "axum-no-default")]
#[doc(hidden)]
pub use ::axum as axum_export;

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

pub mod req_from;
pub mod req_to;
pub use req_from::*;
pub use req_to::*;

mod serverfn;
pub use serverfn::*;

// mod encoding;
// pub use encoding::*;

pub use dioxus_fullstack_hooks::history::provide_fullstack_history_context;

#[doc(inline)]
pub use dioxus_fullstack_hooks::*;

#[doc(inline)]
pub use dioxus_fullstack_macro::*;

/// Implementations of the client side of the server function call.
pub mod client;
pub use client::*;

/// Implementations of the server side of the server function call.
pub mod server;

/// Encodings for arguments and results.
pub mod codec;

#[macro_use]
/// Error types and utilities.
pub mod error;
pub use error::*;

/// Utilities to allow client-side redirects.
pub mod redirect;

/// Types and traits for HTTP responses.
// pub mod response;
pub mod config;
pub mod context;

pub(crate) mod document;
pub(crate) mod ssr;

pub mod prelude {}

mod helpers {
    pub mod sse;
    pub use sse::*;

    pub(crate) mod streaming;

    mod textstream;
    pub use textstream::*;

    pub mod websocket;
    pub use websocket::*;

    pub mod form;
    pub use form::*;

    pub mod state;
    pub use state::*;

    pub mod upload;
    pub use upload::*;
}

pub use helpers::*;
