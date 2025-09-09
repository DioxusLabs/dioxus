use crate::{
    codec::{Patch, Post, Put},
    error::ServerFnError,
    ContentType, Decodes, Encodes, Format, FormatType,
};
use bytes::Bytes;
use serde_lite::{Deserialize, Serialize};

/// Pass arguments and receive responses as JSON in the body of a `POST` request.
pub struct SerdeLiteEncoding;

impl ContentType for SerdeLiteEncoding {
    const CONTENT_TYPE: &'static str = "application/json";
}

impl FormatType for SerdeLiteEncoding {
    const FORMAT_TYPE: Format = Format::Text;
}

impl<T> Encodes<T> for SerdeLiteEncoding
where
    T: Serialize,
{
    type Error = ServerFnError;

    fn encode(value: &T) -> Result<Bytes, Self::Error> {
        serde_json::to_vec(
            &value
                .serialize()
                .map_err(|e| ServerFnError::Serialization(e.to_string()))?,
        )
        .map_err(|e| ServerFnError::Serialization(e.to_string()))
        .map(Bytes::from)
    }
}

impl<T> Decodes<T> for SerdeLiteEncoding
where
    T: Deserialize,
{
    type Error = ServerFnError;

    fn decode(bytes: Bytes) -> Result<T, Self::Error> {
        T::deserialize(
            &serde_json::from_slice(&bytes)
                .map_err(|e| ServerFnError::Deserialization(e.to_string()))?,
        )
        .map_err(|e| ServerFnError::Deserialization(e.to_string()))
    }
}

/// Pass arguments and receive responses as JSON in the body of a `POST` request.
pub type SerdeLite = Post<SerdeLiteEncoding>;

/// Pass arguments and receive responses as JSON in the body of a `PATCH` request.
/// **Note**: Browser support for `PATCH` requests without JS/WASM may be poor.
/// Consider using a `POST` request if functionality without JS/WASM is required.
pub type PatchSerdeLite = Patch<SerdeLiteEncoding>;

/// Pass arguments and receive responses as JSON in the body of a `PUT` request.
/// **Note**: Browser support for `PUT` requests without JS/WASM may be poor.
/// Consider using a `POST` request if functionality without JS/WASM is required.
pub type PutSerdeLite = Put<SerdeLiteEncoding>;
