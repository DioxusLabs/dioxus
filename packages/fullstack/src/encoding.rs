use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};

/// A trait for encoding and decoding data.
///
/// This takes an owned self to make it easier for zero-copy encodings.
pub trait Encoding: 'static {
    fn content_type() -> &'static str;
    fn stream_content_type() -> &'static str;
    fn to_bytes(data: impl Serialize) -> Option<Bytes> {
        let mut buf = Vec::new();
        Self::encode(data, &mut buf)?;
        Some(buf.into())
    }
    fn encode(data: impl Serialize, buf: &mut Vec<u8>) -> Option<usize>;
    fn decode<O: DeserializeOwned>(bytes: Bytes) -> Option<O>;
}

pub struct JsonEncoding;
impl Encoding for JsonEncoding {
    fn content_type() -> &'static str {
        "application/json"
    }
    fn stream_content_type() -> &'static str {
        "application/stream+json"
    }

    fn encode(data: impl Serialize, mut buf: &mut Vec<u8>) -> Option<usize> {
        let len = buf.len();
        serde_json::to_writer(&mut buf, &data).ok()?;
        Some(buf.len() - len)
    }

    fn decode<O: DeserializeOwned>(bytes: Bytes) -> Option<O> {
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

    fn decode<O: DeserializeOwned>(bytes: Bytes) -> Option<O> {
        ciborium::de::from_reader(bytes.as_ref()).ok()
    }

    fn encode(data: impl Serialize, mut buf: &mut Vec<u8>) -> Option<usize> {
        let len = buf.len();
        ciborium::into_writer(&data, &mut buf).ok()?;
        Some(buf.len() - len)
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

    fn encode(data: impl Serialize, mut buf: &mut Vec<u8>) -> Option<usize> {
        let len = buf.len();
        postcard::to_io(&data, &mut buf).ok()?;
        Some(buf.len() - len)
    }

    fn decode<O: DeserializeOwned>(bytes: Bytes) -> Option<O> {
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
    fn encode(data: impl Serialize, buf: &mut Vec<u8>) -> Option<usize> {
        let len = buf.len();
        rmp_serde::encode::write(buf, &data).ok()?;
        Some(buf.len() - len)
    }

    fn decode<O: DeserializeOwned>(bytes: Bytes) -> Option<O> {
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
