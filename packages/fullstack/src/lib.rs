#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(all(feature = "web", feature = "document"))]
mod web;

mod error;
#[doc(hidden)]
pub mod mock_client;

#[cfg(all(feature = "web", feature = "document"))]
pub use web::FullstackWebDocument;

pub use dioxus_fullstack_hooks::history::FullstackHistory;

pub use crate::error::{ServerFnError, ServerFnResult};
#[doc(inline)]
pub use dioxus_fullstack_hooks::*;
#[cfg(feature = "server")]
#[doc(inline)]
pub use dioxus_server::*;
#[doc(inline)]
pub use dioxus_server_macro::*;
pub use server_fn::ServerFn as _;
#[doc(inline)]
pub use server_fn::{
    self, client,
    client::{get_server_url, set_server_url},
    codec, server, BoxedStream, ContentType, Decodes, Encodes, Format, FormatType, ServerFn,
    Websocket,
};
