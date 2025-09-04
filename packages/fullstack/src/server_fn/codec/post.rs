use super::{Encoding, FromReq, FromRes, IntoReq, IntoRes};
use crate::{
    error::{FromServerFnError, IntoAppError, ServerFnErrorErr},
    request::{ClientReq, Req},
    response::{ClientRes, TryRes},
    ContentType, Decodes, Encodes,
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

impl<E, T, Encoding, Request> IntoReq<Post<Encoding>, Request, E> for T
where
    Request: ClientReq<E>,
    Encoding: Encodes<T>,
    E: FromServerFnError,
{
    fn into_req(self, path: &str, accepts: &str) -> Result<Request, E> {
        let data = Encoding::encode(&self).map_err(|e| {
            ServerFnErrorErr::Serialization(e.to_string()).into_app_error()
        })?;
        Request::try_new_post_bytes(path, accepts, Encoding::CONTENT_TYPE, data)
    }
}

impl<E, T, Request, Encoding> FromReq<Post<Encoding>, Request, E> for T
where
    Request: Req<E> + Send + 'static,
    Encoding: Decodes<T>,
    E: FromServerFnError,
{
    async fn from_req(req: Request) -> Result<Self, E> {
        let data = req.try_into_bytes().await?;
        let s = Encoding::decode(data).map_err(|e| {
            ServerFnErrorErr::Deserialization(e.to_string()).into_app_error()
        })?;
        Ok(s)
    }
}

impl<E, Response, Encoding, T> IntoRes<Post<Encoding>, Response, E> for T
where
    Response: TryRes<E>,
    Encoding: Encodes<T>,
    E: FromServerFnError + Send,
    T: Send,
{
    async fn into_res(self) -> Result<Response, E> {
        let data = Encoding::encode(&self).map_err(|e| {
            ServerFnErrorErr::Serialization(e.to_string()).into_app_error()
        })?;
        Response::try_from_bytes(Encoding::CONTENT_TYPE, data)
    }
}

impl<E, Encoding, Response, T> FromRes<Post<Encoding>, Response, E> for T
where
    Response: ClientRes<E> + Send,
    Encoding: Decodes<T>,
    E: FromServerFnError,
{
    async fn from_res(res: Response) -> Result<Self, E> {
        let data = res.try_into_bytes().await?;
        let s = Encoding::decode(data).map_err(|e| {
            ServerFnErrorErr::Deserialization(e.to_string()).into_app_error()
        })?;
        Ok(s)
    }
}
