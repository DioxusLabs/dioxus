use super::Encoding;
// use super::{Encoding, FromReq, FromRes, IntoReq, IntoRes};
use crate::{
    error::{FromServerFnError, IntoAppError, ServerFnError},
    ContentType, Decodes, Encodes, HybridResponse,
};
use std::marker::PhantomData;

/// A codec that encodes the data in the patch body
pub struct Patch<Codec>(PhantomData<Codec>);

impl<Codec: ContentType> ContentType for Patch<Codec> {
    const CONTENT_TYPE: &'static str = Codec::CONTENT_TYPE;
}

impl<Codec: ContentType> Encoding for Patch<Codec> {
    const METHOD: http::Method = http::Method::PATCH;
}

// type Request = crate::HybridRequest;

// impl<T, Encoding> IntoReq<Patch<Encoding>> for T
// where
//     Encoding: Encodes<T>,
// {
//     fn into_req(self, path: &str, accepts: &str) -> Result<Request, HybridError> {
//         let data = Encoding::encode(&self)
//             .map_err(|e| ServerFnError::Serialization(e.to_string()).into_app_error())?;
//         Request::try_new_patch_bytes(path, accepts, Encoding::CONTENT_TYPE, data)
//     }
// }

// impl<T, Encoding> FromReq<Patch<Encoding>> for T
// where
//     Encoding: Decodes<T>,
// {
//     async fn from_req(req: Request) -> Result<Self, HybridError> {
//         let data = req.try_into_bytes().await?;
//         let s = Encoding::decode(data)
//             .map_err(|e| ServerFnError::Deserialization(e.to_string()).into_app_error())?;
//         Ok(s)
//     }
// }

// impl<Encoding, T> IntoRes<Patch<Encoding>> for T
// where
//     Encoding: Encodes<T>,
//     T: Send,
// {
//     async fn into_res(self) -> Result<HybridResponse, HybridError> {
//         let data = Encoding::encode(&self)
//             .map_err(|e| ServerFnError::Serialization(e.to_string()).into_app_error())?;
//         HybridResponse::try_from_bytes(Encoding::CONTENT_TYPE, data)
//     }
// }

// impl<Encoding, T> FromRes<Patch<Encoding>> for T
// where
//     Encoding: Decodes<T>,
// {
//     async fn from_res(res: HybridResponse) -> Result<Self, HybridError> {
//         let data = res.try_into_bytes().await?;
//         let s = Encoding::decode(data)
//             .map_err(|e| ServerFnError::Deserialization(e.to_string()).into_app_error())?;
//         Ok(s)
//     }
// }
