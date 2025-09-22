use axum::{
    body::Bytes,
    extract::{rejection::BytesRejection, FromRequest, Request},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{de::DeserializeOwned, Serialize};

/// A `CBOR`-encoded payload.
#[must_use]
pub struct Cbor<T>(pub T);

fn is_valid_cbor_content_type(headers: &HeaderMap) -> bool {
    let Some(content_type) = headers.get(header::CONTENT_TYPE) else {
        return false;
    };

    let Ok(content_type) = content_type.to_str() else {
        return false;
    };

    let Ok(mime) = content_type.parse::<mime::Mime>() else {
        return false;
    };

    let is_cbor_content_type = mime.type_() == "application"
        && (mime.subtype() == "cbor" || mime.suffix().is_some_and(|name| name == "cbor"));

    is_cbor_content_type
}

impl<S, T> FromRequest<S> for Cbor<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = CborRejection;
    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        if !is_valid_cbor_content_type(req.headers()) {
            return Err(MissingCBorContentType.into());
        }
        let bytes = Bytes::from_request(req, state).await?;
        let value = ciborium::from_reader(&bytes as &[u8]);
        value.map(Cbor).map_err(|_| FailedToParseCbor.into())
    }
}

impl<T> IntoResponse for Cbor<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        let mut buf = Vec::new();
        match ciborium::into_writer(&self.0, &mut buf) {
            Err(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref()),
                )],
                "Failed to serialize".to_string(),
            )
                .into_response(),
            Ok(()) => (
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/cbor"),
                )],
                buf,
            )
                .into_response(),
        }
    }
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
            Err(_) => Err(CborRejection::FailedToParseCbor(FailedToParseCbor)),
        }
    }
}

/// Top-level Errors
#[derive(Debug)]
pub enum CborRejection {
    MissingCBorContentType(MissingCBorContentType),
    BytesRejection(BytesRejection),
    FailedToParseCbor(FailedToParseCbor),
}

impl IntoResponse for CborRejection {
    fn into_response(self) -> Response {
        match self {
            CborRejection::MissingCBorContentType(c) => c.into_response(),
            CborRejection::BytesRejection(b) => b.into_response(),
            CborRejection::FailedToParseCbor(b) => b.into_response(),
        }
    }
}

impl From<BytesRejection> for CborRejection {
    fn from(x: BytesRejection) -> Self {
        CborRejection::BytesRejection(x)
    }
}

#[derive(Debug, Default)]
pub struct FailedToParseCbor;

impl From<FailedToParseCbor> for CborRejection {
    fn from(x: FailedToParseCbor) -> Self {
        CborRejection::FailedToParseCbor(x)
    }
}

impl IntoResponse for FailedToParseCbor {
    fn into_response(self) -> Response {
        (Self::status(), Self::body_text()).into_response()
    }
}

impl FailedToParseCbor {
    /// Get the response body text used for this rejection.
    #[must_use]
    pub fn body_text() -> &'static str {
        "Invalid Request"
    }

    /// Get the status code used for this rejection.
    #[must_use]
    pub fn status() -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

#[derive(Debug, Default)]
pub struct MissingCBorContentType;

impl From<MissingCBorContentType> for CborRejection {
    fn from(x: MissingCBorContentType) -> Self {
        CborRejection::MissingCBorContentType(x)
    }
}

impl IntoResponse for MissingCBorContentType {
    fn into_response(self) -> Response {
        (Self::status(), Self::body_text()).into_response()
    }
}

impl MissingCBorContentType {
    /// Get the response body text used for this rejection.
    #[must_use]
    pub fn body_text() -> &'static str {
        "Expected request with `content-type: application/cbor`"
    }

    /// Get the status code used for this rejection.
    #[must_use]
    pub fn status() -> StatusCode {
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    }
}
