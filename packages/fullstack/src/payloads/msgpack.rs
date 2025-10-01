#![forbid(unsafe_code)]

use axum::{
    body::{Body, Bytes},
    extract::{FromRequest, Request},
    http::{header::HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use axum::{extract::rejection::BytesRejection, http, BoxError};
use derive_more::{Deref, DerefMut, From};
use serde::{de::DeserializeOwned, Serialize};

/// MessagePack Extractor / Response.
///
/// When used as an extractor, it can deserialize request bodies into some type that
/// implements [`serde::Deserialize`]. If the request body cannot be parsed, or value of the
/// `Content-Type` header does not match any of the `application/msgpack`, `application/x-msgpack`
/// or `application/*+msgpack` it will reject the request and return a `400 Bad Request` response.
///
/// When used as a response, it can serialize any type that implements [`serde::Serialize`] to
/// `MsgPack`, and will automatically set `Content-Type: application/msgpack` header.
#[derive(Debug, Clone, Copy, Default, Deref, DerefMut, From)]
pub struct MsgPack<T>(pub T);

impl<T, S> FromRequest<S> for MsgPack<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = MsgPackRejection;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        if !message_pack_content_type(&req) {
            return Err(MsgPackRejection::MissingMsgPackContentType);
        }
        let bytes = Bytes::from_request(req, state).await?;
        let value = rmp_serde::from_slice(&bytes)
            .map_err(|e| MsgPackRejection::InvalidMsgPackBody(e.into()))?;
        Ok(MsgPack(value))
    }
}

impl<T> IntoResponse for MsgPack<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        let bytes = match rmp_serde::encode::to_vec_named(&self.0) {
            Ok(res) => res,
            Err(err) => {
                return Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header("Content-Type", "text/plain")
                    .body(Body::new(err.to_string()))
                    .unwrap();
            }
        };

        let mut res = bytes.into_response();

        res.headers_mut().insert(
            "Content-Type",
            HeaderValue::from_static("application/msgpack"),
        );
        res
    }
}

/// MessagePack Extractor / Response.
///
/// When used as an extractor, it can deserialize request bodies into some type that
/// implements [`serde::Deserialize`]. If the request body cannot be parsed, or value of the
/// `Content-Type` header does not match any of the `application/msgpack`, `application/x-msgpack`
/// or `application/*+msgpack` it will reject the request and return a `400 Bad Request` response.
#[derive(Debug, Clone, Copy, Default, Deref, DerefMut, From)]
pub struct MsgPackRaw<T>(pub T);

impl<T, S> FromRequest<S> for MsgPackRaw<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = MsgPackRejection;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        if !message_pack_content_type(&req) {
            return Err(MsgPackRejection::MissingMsgPackContentType);
        }
        let bytes = Bytes::from_request(req, state).await?;
        let value = rmp_serde::from_slice(&bytes)
            .map_err(|e| MsgPackRejection::InvalidMsgPackBody(e.into()))?;
        Ok(MsgPackRaw(value))
    }
}

impl<T> IntoResponse for MsgPackRaw<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        let bytes = match rmp_serde::encode::to_vec(&self.0) {
            Ok(res) => res,
            Err(err) => {
                return Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header("Content-Type", "text/plain")
                    .body(Body::new(err.to_string()))
                    .unwrap();
            }
        };

        let mut res = bytes.into_response();

        res.headers_mut().insert(
            "Content-Type",
            HeaderValue::from_static("application/msgpack"),
        );
        res
    }
}

fn message_pack_content_type<B>(req: &Request<B>) -> bool {
    let Some(content_type) = req.headers().get("Content-Type") else {
        return false;
    };

    let Ok(content_type) = content_type.to_str() else {
        return false;
    };

    match content_type {
        "application/msgpack" => true,
        "application/x-msgpack" => true,
        ct if ct.starts_with("application/") && ct.ends_with("+msgpack") => true,
        _ => false,
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum MsgPackRejection {
    #[error("Failed to parse the request body as MsgPack: {0}")]
    InvalidMsgPackBody(BoxError),

    #[error("Expected request with `Content-Type: application/msgpack`")]
    MissingMsgPackContentType,

    #[error("Cannot have two request body extractors for a single handler")]
    BodyAlreadyExtracted,

    #[error(transparent)]
    BytesRejection(#[from] BytesRejection),
}

impl IntoResponse for MsgPackRejection {
    fn into_response(self) -> Response {
        match self {
            Self::InvalidMsgPackBody(inner) => {
                let mut res = Response::new(Body::from(format!(
                    "Failed to parse the request body as MsgPack: {}",
                    inner
                )));
                *res.status_mut() = http::StatusCode::BAD_REQUEST;
                res
            }

            Self::MissingMsgPackContentType => {
                let mut res = Response::new(Body::from(
                    "Expected request with `Content-Type: application/msgpack`",
                ));
                *res.status_mut() = http::StatusCode::BAD_REQUEST;
                res
            }

            Self::BodyAlreadyExtracted => {
                let mut res = Response::new(Body::from(
                    "Cannot have two request body extractors for a single handler",
                ));
                *res.status_mut() = http::StatusCode::INTERNAL_SERVER_ERROR;
                res
            }

            Self::BytesRejection(inner) => inner.into_response(),
        }
    }
}
