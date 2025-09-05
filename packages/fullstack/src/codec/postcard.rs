use crate::{
    codec::{Patch, Post, Put},
    ContentType, Decodes, Encodes, Format, FormatType,
};
use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};

/// A codec for Postcard.
pub struct PostcardEncoding;

impl ContentType for PostcardEncoding {
    const CONTENT_TYPE: &'static str = "application/x-postcard";
}

impl FormatType for PostcardEncoding {
    const FORMAT_TYPE: Format = Format::Binary;
}

impl<T> Encodes<T> for PostcardEncoding
where
    T: Serialize,
{
    type Error = postcard::Error;

    fn encode(value: &T) -> Result<Bytes, Self::Error> {
        postcard::to_allocvec(value).map(Bytes::from)
    }
}

impl<T> Decodes<T> for PostcardEncoding
where
    T: DeserializeOwned,
{
    type Error = postcard::Error;

    fn decode(bytes: Bytes) -> Result<T, Self::Error> {
        postcard::from_bytes(&bytes)
    }
}

/// Pass arguments and receive responses with postcard in a `POST` request.
pub type Postcard = Post<PostcardEncoding>;

/// Pass arguments and receive responses with postcard in a `PATCH` request.
/// **Note**: Browser support for `PATCH` requests without JS/WASM may be poor.
/// Consider using a `POST` request if functionality without JS/WASM is required.
pub type PatchPostcard = Patch<PostcardEncoding>;

/// Pass arguments and receive responses with postcard in a `PUT` request.
/// **Note**: Browser support for `PUT` requests without JS/WASM may be poor.
/// Consider using a `POST` request if functionality without JS/WASM is required.
pub type PutPostcard = Put<PostcardEncoding>;
