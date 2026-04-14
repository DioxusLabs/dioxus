use axum::{
    body::Bytes,
    extract::{rejection::BytesRejection, FromRequest},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use postcard::{from_bytes, to_allocvec};
use serde::{de::DeserializeOwned, Serialize};
use std::future::Future;

/// Postcard Extractor / Response.
///
/// When used as an extractor, it can deserialize request bodies into some type that
/// implements [`serde::Deserialize`]. The request will be rejected (and a [`PostcardRejection`] will
/// be returned) if:
///
/// - The request doesn't have a `Content-Type: application/postcard` (or similar) header.
/// - The body doesn't contain syntactically valid Postcard.
/// - The body contains syntactically valid Postcard but it couldn't be deserialized into the target type.
/// - Buffering the request body fails.
///
/// ⚠️ Since parsing Postcard requires consuming the request body, the `Postcard` extractor must be
/// *last* if there are multiple extractors in a handler.
/// See ["the order of extractors"][order-of-extractors]
///
/// [order-of-extractors]: mod@crate::extract#the-order-of-extractors
pub struct Postcard<T>(pub T);

#[derive(thiserror::Error, Debug)]
pub enum PostcardRejection {
    #[error("Expected request with `Content-Type: application/postcard`")]
    MissingPostcardContentType,

    #[error(transparent)]
    PostcardError(#[from] postcard::Error),

    #[error(transparent)]
    Bytes(#[from] BytesRejection),
}

impl IntoResponse for PostcardRejection {
    fn into_response(self) -> Response {
        use PostcardRejection::*;
        // its often easiest to implement `IntoResponse` by calling other implementations
        match self {
            MissingPostcardContentType => {
                (StatusCode::UNSUPPORTED_MEDIA_TYPE, self.to_string()).into_response()
            }
            PostcardError(err) => (StatusCode::BAD_REQUEST, err.to_string()).into_response(),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response(),
        }
    }
}

impl<T, S> FromRequest<S> for Postcard<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = PostcardRejection;

    fn from_request(
        req: axum::extract::Request,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            if postcard_content_type(req.headers()) {
                let bytes = Bytes::from_request(req, state).await?;
                let value = match from_bytes(&bytes) {
                    Ok(value) => value,
                    Err(err) => return Err(PostcardRejection::PostcardError(err)),
                };
                Ok(Postcard(value))
            } else {
                Err(PostcardRejection::MissingPostcardContentType)
            }
        }
    }
}

fn postcard_content_type(headers: &HeaderMap) -> bool {
    let content_type = if let Some(content_type) = headers.get(header::CONTENT_TYPE) {
        content_type
    } else {
        return false;
    };

    let content_type = if let Ok(content_type) = content_type.to_str() {
        content_type
    } else {
        return false;
    };

    let mime = if let Ok(mime) = content_type.parse::<mime::Mime>() {
        mime
    } else {
        return false;
    };

    let is_postcard_content_type = mime.type_() == "application"
        && (mime.subtype() == "postcard" || mime.suffix().is_some_and(|name| name == "postcard"));

    is_postcard_content_type
}

impl<T> IntoResponse for Postcard<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        // TODO: maybe use 128 bytes cause serde is doing something like that
        match to_allocvec(&self.0) {
            Ok(value) => ([(header::CONTENT_TYPE, "application/postcard")], value).into_response(),
            Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
        }
    }
}
