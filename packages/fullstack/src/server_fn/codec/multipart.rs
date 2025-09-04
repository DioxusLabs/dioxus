use super::{Encoding, FromReq};
use crate::{
    error::{FromServerFnError, ServerFnErrorWrapper},
    request::{browser::BrowserFormData, ClientReq, Req},
    ContentType, IntoReq,
};
use futures::StreamExt;
use http::Method;
use multer::Multipart;
use web_sys::FormData;

/// Encodes multipart form data.
///
/// You should primarily use this if you are trying to handle file uploads.
pub struct MultipartFormData;

impl ContentType for MultipartFormData {
    const CONTENT_TYPE: &'static str = "multipart/form-data";
}

impl Encoding for MultipartFormData {
    const METHOD: Method = Method::POST;
}

/// Describes whether the multipart data is on the client side or the server side.
#[derive(Debug)]
pub enum MultipartData {
    /// `FormData` from the browser.
    Client(BrowserFormData),
    /// Generic multipart form using [`multer`]. This implements [`Stream`](futures::Stream).
    Server(multer::Multipart<'static>),
}

impl MultipartData {
    /// Extracts the inner data to handle as a stream.
    ///
    /// On the server side, this always returns `Some(_)`. On the client side, always returns `None`.
    pub fn into_inner(self) -> Option<Multipart<'static>> {
        match self {
            MultipartData::Client(_) => None,
            MultipartData::Server(data) => Some(data),
        }
    }

    /// Extracts the inner form data on the client side.
    ///
    /// On the server side, this always returns `None`. On the client side, always returns `Some(_)`.
    pub fn into_client_data(self) -> Option<BrowserFormData> {
        match self {
            MultipartData::Client(data) => Some(data),
            MultipartData::Server(_) => None,
        }
    }
}

impl From<FormData> for MultipartData {
    fn from(value: FormData) -> Self {
        MultipartData::Client(value.into())
    }
}

impl<E: FromServerFnError, T, Request> IntoReq<MultipartFormData, Request, E>
    for T
where
    Request: ClientReq<E, FormData = BrowserFormData>,
    T: Into<MultipartData>,
{
    fn into_req(self, path: &str, accepts: &str) -> Result<Request, E> {
        let multi = self.into();
        Request::try_new_post_multipart(
            path,
            accepts,
            multi.into_client_data().unwrap(),
        )
    }
}

impl<E, T, Request> FromReq<MultipartFormData, Request, E> for T
where
    Request: Req<E> + Send + 'static,
    T: From<MultipartData>,
    E: FromServerFnError + Send + Sync,
{
    async fn from_req(req: Request) -> Result<Self, E> {
        let boundary = req
            .to_content_type()
            .and_then(|ct| multer::parse_boundary(ct).ok())
            .expect("couldn't parse boundary");
        let stream = req.try_into_stream()?;
        let data = multer::Multipart::new(
            stream.map(|data| data.map_err(|e| ServerFnErrorWrapper(E::de(e)))),
            boundary,
        );
        Ok(MultipartData::Server(data).into())
    }
}
