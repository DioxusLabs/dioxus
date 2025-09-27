use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};

/// A trait for encoding and decoding data.
///
/// This takes an owned self to make it easier for zero-copy encodings.
pub trait Encoding {
    fn content_type() -> &'static str;
    fn stream_content_type() -> &'static str;
    fn to_bytes(data: impl Serialize) -> Option<Bytes>;
    fn from_bytes<O: DeserializeOwned>(bytes: Bytes) -> Option<O>;
}

pub struct JsonEncoding;
impl Encoding for JsonEncoding {
    fn content_type() -> &'static str {
        "application/json"
    }
    fn stream_content_type() -> &'static str {
        "application/stream+json"
    }
    fn to_bytes(data: impl Serialize) -> Option<Bytes> {
        serde_json::to_vec(&data).ok().map(Into::into)
    }

    fn from_bytes<O: DeserializeOwned>(bytes: Bytes) -> Option<O> {
        serde_json::from_slice(&bytes).ok()
    }
}

pub struct CborEncoding;
impl Encoding for CborEncoding {
    fn content_type() -> &'static str {
        "application/cbor"
    }
    fn stream_content_type() -> &'static str {
        "application/stream+cbor"
    }
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
    fn content_type() -> &'static str {
        "application/postcard"
    }
    fn stream_content_type() -> &'static str {
        "application/stream+postcard"
    }
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
    fn content_type() -> &'static str {
        "application/msgpack"
    }
    fn stream_content_type() -> &'static str {
        "application/stream+msgpack"
    }
    fn to_bytes(data: impl Serialize) -> Option<Bytes> {
        rmp_serde::to_vec(&data).ok().map(Into::into)
    }

    fn from_bytes<O: DeserializeOwned>(bytes: Bytes) -> Option<O> {
        rmp_serde::from_slice(&bytes).ok()
    }
}

// todo: ... add rkyv support
// pub struct RkyvEncoding;
// impl Encoding for RkyvEncoding {
//     fn content_type() -> &'static str {
//         "application/rkyv"
//     }
//     fn stream_content_type() -> &'static str {
//         "application/stream+rkyv"
//     }
//     fn to_bytes(data: impl Serialize) -> Option<Bytes> {
//         let mut buf = rkyv::ser::Serializer::new(rkyv::ser::AllocSerializer::new());
//         rkyv::ser::Serializer::serialize(&mut buf, &data).ok()?;
//         Some(Bytes::from(buf.into_inner()))
//     }
//     fn from_bytes<O: DeserializeOwned>(bytes: Bytes) -> Option<O> {
//         let archived = unsafe { rkyv::archived_root::<O>(&bytes) };
//         rkyv::Deserialize::deserialize(archived).ok()
//     }
// }
