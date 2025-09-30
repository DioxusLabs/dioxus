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
pub use headers;
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

pub(crate) mod spawn;
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

// pub mod response;
// pub use response::*;

pub use payloads::*;
pub mod payloads {
    use crate::{ClientRequest, ClientResponse, ClientResult, IntoRequest, ServerFnRejection};
    use crate::{FromResponse, FromResponseParts};
    use axum::extract::{FromRequest, FromRequestParts};
    use axum::response::{IntoResponse, IntoResponseParts, ResponseParts};
    use bytes::Bytes;
    use dioxus_fullstack_core::RequestError;
    use dioxus_fullstack_core::ServerFnError;
    use futures::Stream;
    use headers::Header;
    use http::Method;
    use http::{header::InvalidHeaderValue, HeaderValue};
    use serde::{de::DeserializeOwned, Serialize};
    use std::future::Future;

    pub mod json;
    pub use json::*;

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

    pub mod stream;
    pub use stream::*;

    #[cfg(feature = "ws")]
    pub mod websocket;
    #[cfg(feature = "ws")]
    pub use websocket::*;

    pub mod upload;
    pub use upload::*;

    pub mod redirect;
    pub use redirect::*;

    pub mod ranged;
    pub use ranged::*;

    pub mod response;
    pub use response::*;

    pub mod header;
    pub use header::*;
}

pub use http::{HeaderMap, HeaderValue, Method};

mod client;
pub use client::*;
