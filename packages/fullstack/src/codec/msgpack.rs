use crate::{
    codec::{Patch, Post, Put},
    ContentType, Decodes, Encodes, Format, FormatType,
};
use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};

/// Serializes and deserializes MessagePack with [`rmp_serde`].
pub struct MsgPackEncoding;

impl ContentType for MsgPackEncoding {
    const CONTENT_TYPE: &'static str = "application/msgpack";
}

impl FormatType for MsgPackEncoding {
    const FORMAT_TYPE: Format = Format::Binary;
}

impl<T> Encodes<T> for MsgPackEncoding
where
    T: Serialize,
{
    type Error = rmp_serde::encode::Error;

    fn encode(value: &T) -> Result<Bytes, Self::Error> {
        rmp_serde::to_vec(value).map(Bytes::from)
    }
}

impl<T> Decodes<T> for MsgPackEncoding
where
    T: DeserializeOwned,
{
    type Error = rmp_serde::decode::Error;

    fn decode(bytes: Bytes) -> Result<T, Self::Error> {
        rmp_serde::from_slice(&bytes)
    }
}

/// Pass arguments and receive responses as MessagePack in a `POST` request.
pub type MsgPack = Post<MsgPackEncoding>;

/// Pass arguments and receive responses as MessagePack in the body of a `PATCH` request.
/// **Note**: Browser support for `PATCH` requests without JS/WASM may be poor.
/// Consider using a `POST` request if functionality without JS/WASM is required.
pub type PatchMsgPack = Patch<MsgPackEncoding>;

/// Pass arguments and receive responses as MessagePack in the body of a `PUT` request.
/// **Note**: Browser support for `PUT` requests without JS/WASM may be poor.
/// Consider using a `POST` request if functionality without JS/WASM is required.
pub type PutMsgPack = Put<MsgPackEncoding>;
