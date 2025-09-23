use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};

/// A trait for encoding and decoding data.
///
/// This takes an owned self to make it easier for zero-copy encodings.
pub trait Encoding {
    fn to_bytes(data: impl Serialize) -> Option<Bytes>;
    fn from_bytes<O: DeserializeOwned>(bytes: Bytes) -> Option<O>;
}

pub struct JsonEncoding;
impl Encoding for JsonEncoding {
    fn to_bytes(data: impl Serialize) -> Option<Bytes> {
        serde_json::to_vec(&data).ok().map(Into::into)
    }

    fn from_bytes<O: DeserializeOwned>(bytes: Bytes) -> Option<O> {
        serde_json::from_slice(&bytes).ok()
    }
}

pub struct CborEncoding;
impl Encoding for CborEncoding {
    fn to_bytes(data: impl Serialize) -> Option<Bytes> {
        let mut buf = Vec::new();
        ciborium::into_writer(&data, &mut buf).ok()?;
        Some(buf.into())
    }

    fn from_bytes<O: DeserializeOwned>(bytes: Bytes) -> Option<O> {
        ciborium::de::from_reader(bytes.as_ref()).ok()
    }
}

#[cfg(feature = "postcard")]
pub struct PostcardEncoding;
#[cfg(feature = "postcard")]
impl Encoding for PostcardEncoding {
    fn to_bytes(data: impl Serialize) -> Option<Bytes> {
        postcard::to_allocvec(&data).ok().map(Into::into)
    }

    fn from_bytes<O: DeserializeOwned>(bytes: Bytes) -> Option<O> {
        postcard::from_bytes(bytes.as_ref()).ok()
    }
}

#[cfg(feature = "msgpack")]
pub struct MsgPackEncoding;
#[cfg(feature = "msgpack")]
impl Encoding for MsgPackEncoding {
    fn to_bytes(data: impl Serialize) -> Option<Bytes> {
        rmp_serde::to_vec(&data).ok().map(Into::into)
    }

    fn from_bytes<O: DeserializeOwned>(bytes: Bytes) -> Option<O> {
        rmp_serde::from_slice(&bytes).ok()
    }
}
