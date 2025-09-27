use super::*;
use axum_core::{body::Body, extract::Request};
use http_body_util::BodyExt;

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

impl IntoRequest for FileUpload {
    fn into_request(self, builder: ClientRequest) -> impl Future<Output = ClientResult> + 'static {
        async move { todo!() }
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

impl FromResponse for FileUpload {
    fn from_response(res: ClientResponse) -> impl Future<Output = Result<Self, ServerFnError>> {
        async move { todo!() }
    }
}
