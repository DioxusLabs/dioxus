// #![warn(missing_docs)]
#![allow(clippy::manual_async_fn)]

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

pub mod magic;
pub use magic::*;

pub mod response;
pub use response::*;

pub mod request;
pub use request::*;

pub mod error;
pub use error::*;

pub use http::StatusCode;

pub mod url;
pub use url::*;

pub use payloads::*;
pub mod payloads {
    pub mod jwt;
    pub use jwt::*;

    pub mod json;
    pub use json::*;

    pub mod cbor;
    pub use cbor::*;

    pub mod form;
    pub use form::*;

    pub mod multipart;
    pub mod rkyv;

    #[cfg(feature = "postcard")]
    pub mod postcard;
    #[cfg(feature = "postcard")]
    pub use postcard::*;

    // pub mod msgpack;
    // pub use msgpack::*;

    pub mod text;
    pub use text::*;

    pub mod html;
    pub use html::*;

    pub mod serde_lite;
    pub use serde_lite::*;

    pub mod sse;
    pub use sse::*;

    pub mod textstream;
    pub use textstream::*;

    pub mod websocket;
    pub use websocket::*;

    pub mod upload;
    pub use upload::*;

    pub mod redirect;
    pub use redirect::*;
}
