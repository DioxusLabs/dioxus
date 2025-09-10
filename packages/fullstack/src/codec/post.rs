use super::{Encoding, FromReq, FromRes, IntoReq, IntoRes};
use crate::{
    error::{FromServerFnError, IntoAppError, ServerFnError},
    ContentType, Decodes, Encodes, HybridError, HybridResponse, ServerFnRequestExt,
};
use std::marker::PhantomData;

/// A codec that encodes the data in the post body
pub struct Post<Codec>(PhantomData<Codec>);

impl<Codec: ContentType> ContentType for Post<Codec> {
    const CONTENT_TYPE: &'static str = Codec::CONTENT_TYPE;
}

impl<Codec: ContentType> Encoding for Post<Codec> {
    const METHOD: http::Method = http::Method::POST;
}

type Request = crate::HybridRequest;

impl<T, Encoding> IntoReq<Post<Encoding>> for T
where
    Encoding: Encodes<T>,
{
    fn into_req(self, path: &str, accepts: &str) -> Result<Request, HybridError> {
        let data = Encoding::encode(&self)
            .map_err(|e| ServerFnError::Serialization(e.to_string()).into_app_error())?;
        Request::try_new_post_bytes(path, accepts, Encoding::CONTENT_TYPE, data)
    }
}

impl<T, Encoding> FromReq<Post<Encoding>> for T
where
    Encoding: Decodes<T>,
{
    async fn from_req(req: Request) -> Result<Self, HybridError> {
        let data = req.try_into_bytes().await?;
        let s = Encoding::decode(data)
            .map_err(|e| ServerFnError::Deserialization(e.to_string()).into_app_error())?;
        Ok(s)
    }
}

impl<Encoding, T> IntoRes<Post<Encoding>> for T
where
    Encoding: Encodes<T>,
    T: Send,
{
    async fn into_res(self) -> Result<HybridResponse, HybridError> {
        let data = Encoding::encode(&self)
            .map_err(|e| ServerFnError::Serialization(e.to_string()).into_app_error())?;
        // HybridResponse::try_from_bytes(Encoding::CONTENT_TYPE, data)
        todo!()
    }
}

impl<Encoding, T> FromRes<Post<Encoding>> for T
where
    Encoding: Decodes<T>,
{
    async fn from_res(res: HybridResponse) -> Result<Self, HybridError> {
        let data = res.try_into_bytes().await?;
        let s = Encoding::decode(data)
            .map_err(|e| ServerFnError::Deserialization(e.to_string()).into_app_error())?;
        Ok(s)
    }
}
