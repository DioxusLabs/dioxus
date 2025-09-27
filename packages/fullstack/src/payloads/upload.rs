use super::*;
use axum_core::{body::Body, extract::Request};
use http_body_util::BodyExt;

pub struct FileUpload {
    outgoing_stream: Option<http_body_util::BodyDataStream<Request<Body>>>,
    content_type: Option<String>,
    filename: Option<String>,
}

impl FileUpload {
    // pub fn new()

    pub fn from_stream(filename: String, data: impl Stream<Item = Bytes> + Send + 'static) -> Self {
        todo!()
    }

    pub fn content_type(self, content_type: &str) -> Self {
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
            todo!()
            // let stream = req.into_data_stream();
            // Ok(FileUpload {
            //     outgoing_stream: Some(stream),
            //     content_type: None,
            //     filename: None,
            // })
        }
    }
}

impl FromResponse for FileUpload {
    fn from_response(res: ClientResponse) -> impl Future<Output = Result<Self, ServerFnError>> {
        async move { todo!() }
    }
}
