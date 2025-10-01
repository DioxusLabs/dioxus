// #![warn(missing_docs)]
#![allow(clippy::manual_async_fn)]
#![allow(clippy::needless_return)]

pub use client::{get_server_url, set_server_url};
pub use dioxus_fullstack_core::*;

#[doc(inline)]
pub use dioxus_fullstack_macro::*;

#[cfg(feature = "server")]
pub use axum;

#[cfg(feature = "server")]
pub use inventory;

pub use axum_core;
pub use headers;
pub use http;
pub use reqwest;
pub use serde;

// pub use axum;
// #[doc(hidden)]
// pub use const_format;
// #[doc(hidden)]
// pub use const_str;
// #[doc(hidden)]
// pub use xxhash_rust;

// pub use axum;
// #[doc(hidden)]
// pub use const_format;
// #[doc(hidden)]
// pub use const_str;
// pub use http;
// #[doc(hidden)]
// pub use inventory;
// #[doc(hidden)]
// pub use serde;
// #[doc(hidden)]
// pub use xxhash_rust;

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

pub use payloads::*;
pub mod payloads {
    use crate::{ClientRequest, ClientResponse, ClientResult, IntoRequest, ServerFnRejection};
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

    pub mod rkyv;

    #[cfg(feature = "postcard")]
    pub mod postcard;
    #[cfg(feature = "postcard")]
    pub use postcard::*;

    pub mod msgpack;
    pub use msgpack::*;

    pub mod text;
    pub use text::*;

    pub mod sse;
    pub use sse::*;

    pub mod stream;
    pub use stream::*;

    pub mod upload;
    pub use upload::*;

    pub mod header;
    pub use header::*;

    #[cfg(feature = "ws")]
    pub mod websocket;
    #[cfg(feature = "ws")]
    pub use websocket::*;
}
