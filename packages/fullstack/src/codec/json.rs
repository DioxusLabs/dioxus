use super::{Patch, Post, Put};
use crate::{ContentType, Decodes, Encodes, Format, FormatType};
use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};

/// Serializes and deserializes JSON with [`serde_json`].
pub struct JsonEncoding;

impl ContentType for JsonEncoding {
    const CONTENT_TYPE: &'static str = "application/json";
}

impl FormatType for JsonEncoding {
    const FORMAT_TYPE: Format = Format::Text;
}

impl<T> Encodes<T> for JsonEncoding
where
    T: Serialize,
{
    type Error = serde_json::Error;

    fn encode(output: &T) -> Result<Bytes, Self::Error> {
        serde_json::to_vec(output).map(Bytes::from)
    }
}

impl<T> Decodes<T> for JsonEncoding
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    fn decode(bytes: Bytes) -> Result<T, Self::Error> {
        serde_json::from_slice(&bytes)
    }
}

/// Pass arguments and receive responses as JSON in the body of a `POST` request.
pub type Json = Post<JsonEncoding>;

/// Pass arguments and receive responses as JSON in the body of a `PATCH` request.
/// **Note**: Browser support for `PATCH` requests without JS/WASM may be poor.
/// Consider using a `POST` request if functionality without JS/WASM is required.
pub type PatchJson = Patch<JsonEncoding>;

/// Pass arguments and receive responses as JSON in the body of a `PUT` request.
/// **Note**: Browser support for `PUT` requests without JS/WASM may be poor.
/// Consider using a `POST` request if functionality without JS/WASM is required.
pub type PutJson = Put<JsonEncoding>;
