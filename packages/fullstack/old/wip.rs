use axum::extract::Json;
use axum::extract::OptionalFromRequest;
use axum::extract::{FromRequest, Request};
use axum::response::{IntoResponse, Response};
use bytes::{BufMut, Bytes, BytesMut};
use http::{
    header::{self, HeaderMap, HeaderValue},
    StatusCode,
};
use serde::{de::DeserializeOwned, Serialize};

pub struct Cbor<T>(pub T);

#[derive(Debug)]
pub struct CborRejection;

impl IntoResponse for CborRejection {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, "Invalid CBOR").into_response()
    }
}

impl<T, S> FromRequest<S> for Cbor<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = CborRejection;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        if !cbor_content_type(req.headers()) {
            return Err(CborRejection);
        }

        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(|_| CborRejection)?;
        Self::from_bytes(&bytes)
    }
}

impl<T, S> OptionalFromRequest<S> for Cbor<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = CborRejection;

    async fn from_request(req: Request, state: &S) -> Result<Option<Self>, Self::Rejection> {
        let headers = req.headers();
        if headers.get(header::CONTENT_TYPE).is_some() {
            if cbor_content_type(headers) {
                let bytes = Bytes::from_request(req, state)
                    .await
                    .map_err(|_| CborRejection)?;
                Ok(Some(Self::from_bytes(&bytes)?))
            } else {
                Err(CborRejection)
            }
        } else {
            Ok(None)
        }
    }
}

fn cbor_content_type(headers: &HeaderMap) -> bool {
    let Some(content_type) = headers.get(header::CONTENT_TYPE) else {
        return false;
    };

    let Ok(content_type) = content_type.to_str() else {
        return false;
    };

    content_type == "application/cbor"
}

impl<T> From<T> for Cbor<T> {
    fn from(inner: T) -> Self {
        Self(inner)
    }
}

impl<T> Cbor<T>
where
    T: DeserializeOwned,
{
    /// Construct a `Cbor<T>` from a byte slice.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CborRejection> {
        match ciborium::de::from_reader(bytes) {
            Ok(value) => Ok(Cbor(value)),
            Err(_) => Err(CborRejection),
        }
    }
}

impl<T> IntoResponse for Cbor<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        let mut buf = Vec::new();
        match ciborium::ser::into_writer(&self.0, &mut buf) {
            Ok(()) => (
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/cbor"),
                )],
                buf,
            )
                .into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("text/plain; charset=utf-8"),
                )],
                err.to_string(),
            )
                .into_response(),
        }
    }
}
