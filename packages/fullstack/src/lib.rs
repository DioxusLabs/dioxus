// #![warn(missing_docs)]

pub use dioxus_fullstack_core::client::{get_server_url, set_server_url};
pub use dioxus_fullstack_core::*;

pub use dioxus_fullstack_core::{use_server_cached, use_server_future};

#[doc(inline)]
pub use dioxus_fullstack_macro::*;

#[cfg(feature = "server")]
pub use axum;

#[cfg(feature = "server")]
pub use inventory;

pub use axum_core;
// pub use axum;
// #[doc(hidden)]
// pub use const_format;
// #[doc(hidden)]
// pub use const_str;
pub use http;
pub use reqwest;
pub use serde;
// #[doc(hidden)]
// pub use xxhash_rust;

// pub mod msgpack;

pub mod jwt;
pub use jwt::*;

pub mod magic;
pub use magic::*;

pub mod json;
pub use json::*;

pub mod cbor;
pub mod form;

pub mod multipart;
pub mod postcard;
pub mod rkyv;
pub mod text;
pub use text::*;

pub mod sse;
pub use sse::*;

pub mod textstream;
pub use textstream::*;

pub mod websocket;
pub use websocket::*;

pub mod upload;
pub use upload::*;

pub mod response;
pub use response::*;

pub mod request;
pub use request::*;

pub mod html;
pub use html::*;

pub mod error;
pub use error::*;

pub use http::StatusCode;

pub mod serde_lite;
pub use serde_lite::*;

pub mod url;
pub use url::*;
