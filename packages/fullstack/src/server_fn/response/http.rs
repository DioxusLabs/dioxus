use super::{Res, TryRes};
use crate::error::{
    FromServerFnError, IntoAppError, ServerFnErrorErr, ServerFnErrorWrapper,
    SERVER_FN_ERROR_HEADER,
};
use axum::body::Body;
use bytes::Bytes;
use futures::{Stream, TryStreamExt};
use http::{header, HeaderValue, Response, StatusCode};

impl<E> TryRes<E> for Response<Body>
where
    E: Send + Sync + FromServerFnError,
{
    fn try_from_string(content_type: &str, data: String) -> Result<Self, E> {
        let builder = http::Response::builder();
        builder
            .status(200)
            .header(http::header::CONTENT_TYPE, content_type)
            .body(Body::from(data))
            .map_err(|e| {
                ServerFnErrorErr::Response(e.to_string()).into_app_error()
            })
    }

    fn try_from_bytes(content_type: &str, data: Bytes) -> Result<Self, E> {
        let builder = http::Response::builder();
        builder
            .status(200)
            .header(http::header::CONTENT_TYPE, content_type)
            .body(Body::from(data))
            .map_err(|e| {
                ServerFnErrorErr::Response(e.to_string()).into_app_error()
            })
    }

    fn try_from_stream(
        content_type: &str,
        data: impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static,
    ) -> Result<Self, E> {
        let body =
            Body::from_stream(data.map_err(|e| ServerFnErrorWrapper(E::de(e))));
        let builder = http::Response::builder();
        builder
            .status(200)
            .header(http::header::CONTENT_TYPE, content_type)
            .body(body)
            .map_err(|e| {
                ServerFnErrorErr::Response(e.to_string()).into_app_error()
            })
    }
}

impl Res for Response<Body> {
    fn error_response(path: &str, err: Bytes) -> Self {
        Response::builder()
            .status(http::StatusCode::INTERNAL_SERVER_ERROR)
            .header(SERVER_FN_ERROR_HEADER, path)
            .body(err.into())
            .unwrap()
    }

    fn redirect(&mut self, path: &str) {
        if let Ok(path) = HeaderValue::from_str(path) {
            self.headers_mut().insert(header::LOCATION, path);
            *self.status_mut() = StatusCode::FOUND;
        }
    }
}
