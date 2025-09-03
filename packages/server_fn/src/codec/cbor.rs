use super::{Patch, Post, Put};
use crate::{ContentType, Decodes, Encodes, Format, FormatType};
use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};

/// Serializes and deserializes CBOR with [`ciborium`].
pub struct CborEncoding;

impl ContentType for CborEncoding {
    const CONTENT_TYPE: &'static str = "application/cbor";
}

impl FormatType for CborEncoding {
    const FORMAT_TYPE: Format = Format::Binary;
}

impl<T> Encodes<T> for CborEncoding
where
    T: Serialize,
{
    type Error = ciborium::ser::Error<std::io::Error>;

    fn encode(value: &T) -> Result<Bytes, Self::Error> {
        let mut buffer: Vec<u8> = Vec::new();
        ciborium::ser::into_writer(value, &mut buffer)?;
        Ok(Bytes::from(buffer))
    }
}

impl<T> Decodes<T> for CborEncoding
where
    T: DeserializeOwned,
{
    type Error = ciborium::de::Error<std::io::Error>;

    fn decode(bytes: Bytes) -> Result<T, Self::Error> {
        ciborium::de::from_reader(bytes.as_ref())
    }
}

/// Pass arguments and receive responses using `cbor` in a `POST` request.
pub type Cbor = Post<CborEncoding>;

/// Pass arguments and receive responses using `cbor` in the body of a `PATCH` request.
/// **Note**: Browser support for `PATCH` requests without JS/WASM may be poor.
/// Consider using a `POST` request if functionality without JS/WASM is required.
pub type PatchCbor = Patch<CborEncoding>;

/// Pass arguments and receive responses using `cbor` in the body of a `PUT` request.
/// **Note**: Browser support for `PUT` requests without JS/WASM may be poor.
/// Consider using a `POST` request if functionality without JS/WASM is required.
pub type PutCbor = Put<CborEncoding>;
