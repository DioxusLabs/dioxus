use std::prelude::rust_2024::Future;

use axum_core::{
    body::Body,
    extract::{FromRequest, Request},
};
use bytes::Bytes;
use futures::Stream;
use http_body_util::BodyExt;

use crate::ServerFnRejection;

pub struct FileUpload {
    outgoing_stream: Option<http_body_util::BodyDataStream<Request<Body>>>,
}

impl FileUpload {
    pub fn from_stream(
        filename: String,
        content_type: String,
        data: impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static,
    ) -> Self {
        todo!()
    }
}

impl<S> FromRequest<S> for FileUpload {
    type Rejection = ServerFnRejection;

    fn from_request(
        req: Request,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let stream = req.into_data_stream();
            Ok(FileUpload {
                outgoing_stream: Some(stream),
            })
        }
    }
}
