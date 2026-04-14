// #![warn(missing_docs)]
#![allow(clippy::manual_async_fn)]
#![allow(clippy::needless_return)]

pub use client::{get_server_url, set_server_url};
pub use dioxus_fullstack_core::*;

#[doc(inline)]
pub use dioxus_fullstack_macro::*;

pub use axum_core;
pub use headers;
pub use http;
pub use reqwest;
pub use serde;

/// Re-export commonly used items from axum, http, and hyper for convenience.
pub use axum::{body, extract, response, routing};

#[doc(hidden)]
pub use const_format;
#[doc(hidden)]
pub use const_str;
#[doc(hidden)]
pub use xxhash_rust;

#[cfg(feature = "server")]
pub use {axum, axum_extra::TypedHeader, inventory};

#[cfg(feature = "server")]
pub(crate) mod spawn;
#[cfg(feature = "server")]
pub(crate) use spawn::*;

pub mod magic;
pub use magic::*;

pub mod request;
pub use request::*;

pub use http::StatusCode;

pub mod encoding;
pub use encoding::*;

pub mod lazy;
pub use lazy::*;

pub use http::{HeaderMap, HeaderValue, Method};

mod client;
pub use client::*;

pub use axum::extract::Json;
pub use axum::response::{NoContent, Redirect};

pub use crate::request::{FromResponse, FromResponseParts};

pub use payloads::*;
pub mod payloads {
    use crate::{ClientRequest, ClientResponse, ClientResult, IntoRequest};
    use crate::{FromResponse, FromResponseParts};
    use axum::extract::FromRequest;
    use axum::response::{IntoResponse, IntoResponseParts, ResponseParts};
    use bytes::Bytes;
    use dioxus_fullstack_core::ServerFnError;
    use futures::Stream;
    use headers::Header;
    use http::{header::InvalidHeaderValue, HeaderValue};
    use serde::{de::DeserializeOwned, Serialize};
    use std::future::Future;

    mod axum_types;

    pub mod cbor;
    pub use cbor::*;

    pub mod form;
    pub use form::*;

    pub mod multipart;
    pub use multipart::*;

    #[cfg(feature = "postcard")]
    pub mod postcard;

    #[cfg(feature = "postcard")]
    pub use postcard::*;

    #[cfg(feature = "msgpack")]
    pub mod msgpack;
    #[cfg(feature = "msgpack")]
    pub use msgpack::*;

    pub mod text;
    pub use text::*;

    pub mod sse;
    pub use sse::*;

    pub mod stream;
    pub use stream::*;

    pub mod files;
    pub use files::*;

    pub mod header;
    pub use header::*;

    pub mod query;
    pub use query::*;

    #[cfg(feature = "ws")]
    pub mod websocket;
    #[cfg(feature = "ws")]
    pub use websocket::*;
}
