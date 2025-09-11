#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
// #![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unexpected_cfgs)]

pub mod req_from;
pub mod req_to;
pub use req_from::*;
pub use req_to::*;

pub use axum;
pub use http;
#[doc(hidden)]
pub use inventory;

pub use fetch::*;
pub mod fetch;
pub mod protocols;
pub use protocols::*;

mod textstream;
pub use textstream::*;

pub mod websocket;
pub use websocket::*;

mod old;
pub mod state;

mod serverfn;
pub use serverfn::*;

mod encoding;
pub use encoding::*;

pub use dioxus_fullstack_hooks::history::provide_fullstack_history_context;

#[doc(inline)]
pub use dioxus_fullstack_hooks::*;

#[doc(inline)]
pub use dioxus_fullstack_macro::*;
// pub use ServerFn as _;

/// Implementations of the client side of the server function call.
pub mod client;

/// Implementations of the server side of the server function call.
pub mod server;

/// Encodings for arguments and results.
pub mod codec;

#[macro_use]
/// Error types and utilities.
pub mod error;

/// Utilities to allow client-side redirects.
pub mod redirect;
/// Types and traits for  for HTTP requests.
pub mod request;
pub use request::ServerFnRequestExt;

/// Types and traits for HTTP responses.
pub mod response;

// re-exported to make it possible to implement a custom Client without adding a separate
// dependency on `bytes`
pub use bytes::Bytes;
pub use client::{get_server_url, set_server_url};
pub use error::{FromServerFnError, ServerFnError, ServerFnResult};
#[doc(hidden)]
pub use xxhash_rust;

#[doc(hidden)]
pub use const_format;
#[doc(hidden)]
pub use const_str;
// #![deny(missing)]

// #[doc(hidden)]
// #[cfg(feature = "serde-lite")]
// pub use serde_lite;

// pub mod server_fn {
//     // pub use crate::{
//     //     client,
//     //     client::{get_server_url, set_server_url},
//     //     codec, server, BoxedStream, ContentType, Decodes, Encodes, Format, FormatType, ServerFn,
//     //     Websocket,
//     // };
//     pub use serde;
// }

// pub(crate) use crate::client::Client;
// pub(crate) use crate::server::Server;

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

#[doc(hidden)]
pub use serde;

pub mod prelude {
    use dioxus_core::RenderError;
    use dioxus_hooks::Resource;
    use dioxus_signals::{Loader, Signal};
    use std::{marker::PhantomData, prelude::rust_2024::Future};

    pub use crate::layer;
    pub use crate::middleware;
    pub use http::Request;

    pub fn use_loader<F: Future<Output = anyhow::Result<T>>, T: 'static>(
        // pub fn use_loader<F: Future<Output = Result<T, E>>, T: 'static, E: Into<anyhow::Error>>(
        f: impl FnMut() -> F,
    ) -> Result<Loader<T>, RenderError> {
        todo!()
    }

    pub struct ServerState<T> {
        _t: PhantomData<*const T>,
    }

    impl<T> ServerState<T> {
        fn get(&self) -> &T {
            todo!()
        }

        pub const fn new(f: fn() -> T) -> Self {
            Self { _t: PhantomData }
        }
    }

    impl<T> std::ops::Deref for ServerState<T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            todo!()
        }
    }
    impl<T> std::ops::DerefMut for ServerState<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            todo!()
        }
    }

    unsafe impl<T> Send for ServerState<T> {}
    unsafe impl<T> Sync for ServerState<T> {}
}

pub mod config;
pub mod context;

pub(crate) mod document;
pub(crate) mod render;
pub(crate) mod streaming;

pub(crate) use config::*;

pub use crate::config::{ServeConfig, ServeConfigBuilder};
pub use crate::context::Axum;
pub use crate::server::*;
pub use config::*;
pub use context::{
    extract, server_context, with_server_context, DioxusServerContext, FromContext,
    FromServerContext, ProvideServerContext,
};
pub use dioxus_isrg::{IncrementalRenderer, IncrementalRendererConfig};
pub use document::ServerDocument;

#[cfg(not(target_arch = "wasm32"))]
mod launch;

#[cfg(not(target_arch = "wasm32"))]
pub use launch::{launch, launch_cfg};

#[macro_export]
macro_rules! make_server_fn {
    (
        #[$method:ident($path:literal)]
        pub async fn $name:ident ( $( $arg_name:ident : $arg_ty:ty ),* $(,)? ) -> $ret:ty $body:block
    ) => {
        pub async fn $name( $( $arg_name : $arg_ty ),* ) -> $ret {
            // If no server feature, we always make a request to the server
            if cfg!(not(feature = "server")) {
                return Ok(dioxus_fullstack::fetch::fetch("/thing")
                    .method("POST")
                    // .json(&serde_json::json!({ "a": a, "b": b }))
                    .send()
                    .await?
                    .json::<()>()
                    .await?);
            }

            // if we do have the server feature, we can run the code directly
            #[cfg(feature = "server")]
            {
                async fn run_user_code(
                    $( $arg_name : $arg_ty ),*
                ) -> $ret {
                    $body
                }

                inventory::submit! {
                    ServerFunction::new(
                        http::Method::GET,
                        "/thing",
                        |req| {
                            Box::pin(async move {
                                todo!()
                            })
                        },
                        None
                    )
                }

                return run_user_code(
                    $( $arg_name ),*
                ).await;
            }

            #[allow(unreachable_code)]
            {
                unreachable!()
            }
        }
    };
}
