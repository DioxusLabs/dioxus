use serde::{de::DeserializeOwned, Serialize};

pub trait Encoding {
    fn to_bytes(data: &impl Serialize) -> Option<Vec<u8>>;
    fn from_bytes<O: DeserializeOwned>(bytes: &[u8]) -> Option<O>;
}

pub struct JsonEncoding;
impl Encoding for JsonEncoding {
    fn to_bytes(data: &impl Serialize) -> Option<Vec<u8>> {
        serde_json::to_vec(data).ok()
    }

    fn from_bytes<O: DeserializeOwned>(bytes: &[u8]) -> Option<O> {
        serde_json::from_slice(bytes).ok()
    }
}

pub struct CborEncoding;
impl Encoding for CborEncoding {
    fn to_bytes(data: &impl Serialize) -> Option<Vec<u8>> {
        let mut buf = Vec::new();
        ciborium::into_writer(data, &mut buf).ok()?;
        Some(buf)
    }

    fn from_bytes<O: DeserializeOwned>(bytes: &[u8]) -> Option<O> {
        ciborium::de::from_reader(bytes).ok()
    }
}

#[cfg(feature = "postcard")]
pub struct PostcardEncoding;
#[cfg(feature = "postcard")]
impl Encoding for PostcardEncoding {
    fn to_bytes(data: &impl Serialize) -> Option<Vec<u8>> {
        postcard::to_allocvec(data).ok()
    }

    fn from_bytes<O: DeserializeOwned>(bytes: &[u8]) -> Option<O> {
        postcard::from_bytes(bytes).ok()
    }
}

#[cfg(feature = "msgpack")]
pub struct MsgPackEncoding;
#[cfg(feature = "msgpack")]
impl Encoding for MsgPackEncoding {
    fn to_bytes(data: &impl Serialize) -> Option<Vec<u8>> {
        rmp_serde::to_vec(data).ok()
    }

    fn from_bytes<O: DeserializeOwned>(bytes: &[u8]) -> Option<O> {
        rmp_serde::from_slice(bytes).ok()
    }
}
