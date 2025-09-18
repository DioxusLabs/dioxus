// #![warn(missing_docs)]

// impl From<reqwest::Error> for ServerFnError {
//     fn from(value: reqwest::Error) -> Self {
//         ServerFnError::Request {
//             message: value.to_string(),
//             code: value.status().map(|s| s.as_u16()),
//         }
//     }
// }

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

pub mod error;
pub use error::*;

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

pub mod request;
pub use request::*;

pub mod response;
pub use response::*;
