use axum::{
    body::Bytes,
    extract::{rejection::BytesRejection, FromRequest, Request},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{de::DeserializeOwned, Serialize};

/// CBOR Extractor / Response.
///
/// When used as an extractor, it can deserialize request bodies into some type that
/// implements [`serde::Deserialize`]. The request will be rejected (and a [`CborRejection`] will
/// be returned) if:
///
/// - The request doesn't have a `Content-Type: application/cbor` (or similar) header.
/// - The body doesn't contain syntactically valid CBOR.
/// - The body contains syntactically valid CBOR but it couldn't be deserialized into the target type.
/// - Buffering the request body fails.
///
/// ⚠️ Since parsing CBOR requires consuming the request body, the `Cbor` extractor must be
/// *last* if there are multiple extractors in a handler.
/// See ["the order of extractors"][order-of-extractors]
///
/// [order-of-extractors]: mod@crate::extract#the-order-of-extractors
#[must_use]
pub struct Cbor<T>(pub T);

/// Check if the request has a valid CBOR content type header.
///
/// This function validates that the `Content-Type` header is set to `application/cbor`
/// or a compatible CBOR media type (including subtypes with `+cbor` suffix).
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

    /// Extract a CBOR payload from the request body.
    ///
    /// This implementation validates the content type and deserializes the CBOR data.
    /// Returns a `CborRejection` if validation or deserialization fails.
    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        if !is_valid_cbor_content_type(req.headers()) {
            return Err(CborRejection::MissingCborContentType);
        }
        let bytes = Bytes::from_request(req, state).await?;
        let value =
            ciborium::from_reader(&bytes as &[u8]).map_err(|_| CborRejection::FailedToParseCbor)?;
        Ok(Cbor(value))
    }
}

impl<T> IntoResponse for Cbor<T>
where
    T: Serialize,
{
    /// Convert the CBOR payload into an HTTP response.
    ///
    /// This serializes the inner value to CBOR format and sets the appropriate
    /// `Content-Type: application/cbor` header. Returns a 500 Internal Server Error
    /// if serialization fails.
    fn into_response(self) -> Response {
        let mut buf = Vec::new();
        match ciborium::into_writer(&self.0, &mut buf) {
            Err(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref()),
                )],
                "Failed to serialize to CBOR".to_string(),
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
    /// Create a `Cbor<T>` from the inner value.
    ///
    /// This is a convenience constructor that wraps any value in the `Cbor` struct.
    fn from(inner: T) -> Self {
        Self(inner)
    }
}

impl<T> Cbor<T>
where
    T: DeserializeOwned,
{
    /// Construct a `Cbor<T>` from a byte slice.
    ///
    /// This method attempts to deserialize the provided bytes as CBOR data.
    /// Returns a `CborRejection` if deserialization fails.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CborRejection> {
        ciborium::de::from_reader(bytes)
            .map(Cbor)
            .map_err(|_| CborRejection::FailedToParseCbor)
    }
}

/// Rejection type for CBOR extraction failures.
///
/// This enum represents the various ways that CBOR extraction can fail.
/// It implements `IntoResponse` to provide appropriate HTTP responses for each error type.
#[derive(thiserror::Error, Debug)]
pub enum CborRejection {
    /// The request is missing the required `Content-Type: application/cbor` header.
    #[error("Expected request with `Content-Type: application/cbor`")]
    MissingCborContentType,

    /// Failed to parse the request body as valid CBOR.
    #[error("Invalid CBOR data")]
    FailedToParseCbor,

    /// Failed to read the request body bytes.
    #[error(transparent)]
    BytesRejection(#[from] BytesRejection),
}

impl IntoResponse for CborRejection {
    fn into_response(self) -> Response {
        use CborRejection::*;
        match self {
            MissingCborContentType => {
                (StatusCode::UNSUPPORTED_MEDIA_TYPE, self.to_string()).into_response()
            }
            FailedToParseCbor => (StatusCode::BAD_REQUEST, self.to_string()).into_response(),
            BytesRejection(rejection) => rejection.into_response(),
        }
    }
}
