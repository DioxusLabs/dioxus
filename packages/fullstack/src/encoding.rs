use base64::{engine::general_purpose::STANDARD_NO_PAD, DecodeError, Engine};

use crate::{FromServerFnError, ServerFnError};

// use super::client::Client;
// use super::codec::{Encoding, FromReq, FromRes, IntoReq, IntoRes};

// #[cfg(feature = "form-redirects")]
// use super::error::ServerFnUrlError;

use super::middleware::{BoxedService, Layer, Service};
use super::redirect::call_redirect_hook;
// use super::response::{Res, TryRes};
// use super::response::{ClientRes, Res, TryRes};
use bytes::{BufMut, Bytes, BytesMut};
use dashmap::DashMap;
use futures::{pin_mut, SinkExt, Stream, StreamExt};
use http::Method;

// use super::server::Server;
use std::{
    fmt::{Debug, Display},
    future::Future,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    pin::Pin,
    sync::{Arc, LazyLock},
};

/// A trait for types that can be encoded into a bytes for a request body.
pub trait Encodes<T>: ContentType + FormatType {
    /// The error type that can be returned if the encoding fails.
    type Error: Display + Debug;

    /// Encodes the given value into a bytes.
    fn encode(output: &T) -> Result<Bytes, Self::Error>;
}

/// A trait for types that can be decoded from a bytes for a response body.
pub trait Decodes<T> {
    /// The error type that can be returned if the decoding fails.
    type Error: Display;

    /// Decodes the given bytes into a value.
    fn decode(bytes: Bytes) -> Result<T, Self::Error>;
}

/// Encode format type
pub enum Format {
    /// Binary representation
    Binary,

    /// utf-8 compatible text representation
    Text,
}

/// A trait for types with an associated content type.
pub trait ContentType {
    /// The MIME type of the data.
    const CONTENT_TYPE: &'static str;
}

/// Data format representation
pub trait FormatType {
    /// The representation type
    const FORMAT_TYPE: Format;

    /// Encodes data into a string.
    fn into_encoded_string(bytes: Bytes) -> String {
        match Self::FORMAT_TYPE {
            Format::Binary => STANDARD_NO_PAD.encode(bytes),
            Format::Text => String::from_utf8(bytes.into())
                .expect("Valid text format type with utf-8 comptabile string"),
        }
    }

    /// Decodes string to bytes
    fn from_encoded_string(data: &str) -> Result<Bytes, DecodeError> {
        match Self::FORMAT_TYPE {
            Format::Binary => STANDARD_NO_PAD.decode(data).map(|data| data.into()),
            Format::Text => Ok(Bytes::copy_from_slice(data.as_bytes())),
        }
    }
}

// Serializes a Result<Bytes, Bytes> into a single Bytes instance.
// Format: [tag: u8][content: Bytes]
// - Tag 0: Ok variant
// - Tag 1: Err variant
pub fn serialize_result(result: Result<Bytes, Bytes>) -> Bytes {
    match result {
        Ok(bytes) => {
            let mut buf = BytesMut::with_capacity(1 + bytes.len());
            buf.put_u8(0); // Tag for Ok variant
            buf.extend_from_slice(&bytes);
            buf.freeze()
        }
        Err(bytes) => {
            let mut buf = BytesMut::with_capacity(1 + bytes.len());
            buf.put_u8(1); // Tag for Err variant
            buf.extend_from_slice(&bytes);
            buf.freeze()
        }
    }
}

// Deserializes a Bytes instance back into a Result<Bytes, Bytes>.
pub fn deserialize_result<E: FromServerFnError>(bytes: Bytes) -> Result<Bytes, Bytes> {
    if bytes.is_empty() {
        return Err(E::from_server_fn_error(ServerFnError::Deserialization(
            "Data is empty".into(),
        ))
        .ser());
    }

    let tag = bytes[0];
    let content = bytes.slice(1..);

    match tag {
        0 => Ok(content),
        1 => Err(content),
        _ => Err(E::from_server_fn_error(ServerFnError::Deserialization(
            "Invalid data tag".into(),
        ))
        .ser()), // Invalid tag
    }
}
