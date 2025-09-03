use super::{Res, TryRes};
use crate::error::{
    FromServerFnError, ServerFnErrorWrapper, SERVER_FN_ERROR_HEADER,
};
use actix_web::{
    http::{
        header,
        header::{HeaderValue, LOCATION},
        StatusCode,
    },
    HttpResponse,
};
use bytes::Bytes;
use futures::{Stream, StreamExt};
use send_wrapper::SendWrapper;

/// A wrapped Actix response.
///
/// This uses a [`SendWrapper`] that allows the Actix `HttpResponse` type to be `Send`, but panics
/// if it it is ever sent to another thread. Actix pins request handling to a single thread, so this
/// is necessary to be compatible with traits that require `Send` but should never panic in actual use.
pub struct ActixResponse(pub(crate) SendWrapper<HttpResponse>);

impl ActixResponse {
    /// Returns the raw Actix response.
    pub fn take(self) -> HttpResponse {
        self.0.take()
    }
}

impl From<HttpResponse> for ActixResponse {
    fn from(value: HttpResponse) -> Self {
        Self(SendWrapper::new(value))
    }
}

impl<E> TryRes<E> for ActixResponse
where
    E: FromServerFnError,
{
    fn try_from_string(content_type: &str, data: String) -> Result<Self, E> {
        let mut builder = HttpResponse::build(StatusCode::OK);
        Ok(ActixResponse(SendWrapper::new(
            builder
                .insert_header((header::CONTENT_TYPE, content_type))
                .body(data),
        )))
    }

    fn try_from_bytes(content_type: &str, data: Bytes) -> Result<Self, E> {
        let mut builder = HttpResponse::build(StatusCode::OK);
        Ok(ActixResponse(SendWrapper::new(
            builder
                .insert_header((header::CONTENT_TYPE, content_type))
                .body(data),
        )))
    }

    fn try_from_stream(
        content_type: &str,
        data: impl Stream<Item = Result<Bytes, Bytes>> + 'static,
    ) -> Result<Self, E> {
        let mut builder = HttpResponse::build(StatusCode::OK);
        Ok(ActixResponse(SendWrapper::new(
            builder
                .insert_header((header::CONTENT_TYPE, content_type))
                .streaming(data.map(|data| {
                    data.map_err(|e| ServerFnErrorWrapper(E::de(e)))
                })),
        )))
    }
}

impl Res for ActixResponse {
    fn error_response(path: &str, err: Bytes) -> Self {
        ActixResponse(SendWrapper::new(
            HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
                .append_header((SERVER_FN_ERROR_HEADER, path))
                .body(err),
        ))
    }

    fn redirect(&mut self, path: &str) {
        if let Ok(path) = HeaderValue::from_str(path) {
            *self.0.status_mut() = StatusCode::FOUND;
            self.0.headers_mut().insert(LOCATION, path);
        }
    }
}
