#![forbid(unsafe_code)]

use axum::{
    body::{Body, Bytes},
    extract::{FromRequest, Request},
    http::{header::HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use axum::{extract::rejection::BytesRejection, http, BoxError};
use derive_more::{Deref, DerefMut, From};
use hyper::header;
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
            return Err(MissingMsgPackContentType.into());
        }
        let bytes = Bytes::from_request(req, state).await?;
        let value = rmp_serde::from_slice(&bytes).map_err(InvalidMsgPackBody::from_err)?;
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
                    .header(header::CONTENT_TYPE, "text/plain")
                    .body(Body::new(err.to_string()))
                    .unwrap();
            }
        };

        let mut res = bytes.into_response();

        res.headers_mut().insert(
            header::CONTENT_TYPE,
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
            return Err(MissingMsgPackContentType.into());
        }
        let bytes = Bytes::from_request(req, state).await?;
        let value = rmp_serde::from_slice(&bytes).map_err(InvalidMsgPackBody::from_err)?;
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
                    .header(header::CONTENT_TYPE, "text/plain")
                    .body(Body::new(err.to_string()))
                    .unwrap();
            }
        };

        let mut res = bytes.into_response();

        res.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/msgpack"),
        );
        res
    }
}

fn message_pack_content_type<B>(req: &Request<B>) -> bool {
    let Some(content_type) = req.headers().get(header::CONTENT_TYPE) else {
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

#[derive(Debug)]
#[non_exhaustive]
pub struct InvalidMsgPackBody(BoxError);

impl InvalidMsgPackBody {
    pub(crate) fn from_err<E>(err: E) -> Self
    where
        E: Into<BoxError>,
    {
        Self(err.into())
    }
}

impl IntoResponse for InvalidMsgPackBody {
    fn into_response(self) -> Response {
        let mut res = Response::new(Body::from(format!(
            "Failed to parse the request body as MsgPack: {}",
            self.0
        )));
        *res.status_mut() = http::StatusCode::BAD_REQUEST;
        res
    }
}

impl std::fmt::Display for InvalidMsgPackBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to parse the request body as MsgPack")
    }
}

impl std::error::Error for InvalidMsgPackBody {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.0)
    }
}

#[derive(Debug)]
#[non_exhaustive]
/// Rejection type for [`MsgPack`](super::MsgPack) used if the `Content-Type`
/// header is missing
pub struct MissingMsgPackContentType;

impl IntoResponse for MissingMsgPackContentType {
    fn into_response(self) -> Response {
        let mut res = Response::new(Body::from(
            "Expected request with `Content-Type: application/msgpack`",
        ));
        *res.status_mut() = http::StatusCode::BAD_REQUEST;
        res
    }
}

impl std::error::Error for MissingMsgPackContentType {}
impl std::fmt::Display for MissingMsgPackContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Expected request with `Content-Type: application/msgpack`"
        )
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub struct BodyAlreadyExtracted;
impl IntoResponse for BodyAlreadyExtracted {
    fn into_response(self) -> Response {
        let mut res = Response::new(Body::from(
            "Cannot have two request body extractors for a single handler",
        ));
        *res.status_mut() = http::StatusCode::INTERNAL_SERVER_ERROR;
        res
    }
}
impl std::fmt::Display for BodyAlreadyExtracted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Cannot have two request body extractors for a single handler"
        )
    }
}
impl std::error::Error for BodyAlreadyExtracted {}

#[derive(Debug)]
#[non_exhaustive]
pub enum MsgPackRejection {
    InvalidMsgPackBody(InvalidMsgPackBody),
    MissingMsgPackContentType(MissingMsgPackContentType),
    BodyAlreadyExtracted(BodyAlreadyExtracted),
    BytesRejection(BytesRejection),
}

impl IntoResponse for MsgPackRejection {
    fn into_response(self) -> Response {
        match self {
            Self::InvalidMsgPackBody(inner) => inner.into_response(),
            Self::MissingMsgPackContentType(inner) => inner.into_response(),
            Self::BodyAlreadyExtracted(inner) => inner.into_response(),
            Self::BytesRejection(inner) => inner.into_response(),
        }
    }
}

impl From<InvalidMsgPackBody> for MsgPackRejection {
    fn from(inner: InvalidMsgPackBody) -> Self {
        Self::InvalidMsgPackBody(inner)
    }
}

impl From<BytesRejection> for MsgPackRejection {
    fn from(inner: BytesRejection) -> Self {
        Self::BytesRejection(inner)
    }
}

impl From<MissingMsgPackContentType> for MsgPackRejection {
    fn from(inner: MissingMsgPackContentType) -> Self {
        Self::MissingMsgPackContentType(inner)
    }
}

impl From<BodyAlreadyExtracted> for MsgPackRejection {
    fn from(inner: BodyAlreadyExtracted) -> Self {
        Self::BodyAlreadyExtracted(inner)
    }
}

impl std::fmt::Display for MsgPackRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMsgPackBody(inner) => write!(f, "{}", inner),
            Self::MissingMsgPackContentType(inner) => write!(f, "{}", inner),
            Self::BodyAlreadyExtracted(inner) => write!(f, "{}", inner),
            Self::BytesRejection(inner) => write!(f, "{}", inner),
        }
    }
}

impl std::error::Error for MsgPackRejection {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidMsgPackBody(inner) => Some(inner),
            Self::MissingMsgPackContentType(inner) => Some(inner),
            Self::BodyAlreadyExtracted(inner) => Some(inner),
            Self::BytesRejection(inner) => Some(inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, extract::FromRequest, http::HeaderValue, response::IntoResponse};
    use futures_util::StreamExt;

    use super::{MsgPack, MsgPackRaw, MsgPackRejection};
    use hyper::{header, Request};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Input {
        foo: String,
    }

    fn into_request<T: Serialize>(value: &T) -> Request<Body> {
        let serialized =
            rmp_serde::encode::to_vec_named(&value).expect("Failed to serialize test struct");

        let body = Body::from(serialized);
        Request::new(body)
    }

    fn into_request_raw<T: Serialize>(value: &T) -> Request<Body> {
        let serialized =
            rmp_serde::encode::to_vec(&value).expect("Failed to serialize test struct");

        let body = Body::from(serialized);
        Request::new(body)
    }

    #[tokio::test]
    async fn serializes_named() {
        let input = Input { foo: "bar".into() };
        let serialized = rmp_serde::encode::to_vec_named(&input);
        assert!(serialized.is_ok());
        let serialized = serialized.unwrap();

        let body = MsgPack(input).into_response().into_body();
        let bytes = to_bytes(body).await;

        assert_eq!(serialized, bytes);
    }

    #[tokio::test]
    async fn deserializes_named() {
        let input = Input { foo: "bar".into() };
        let mut request = into_request(&input);

        request.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/msgpack"),
        );

        let outcome = <MsgPack<Input> as FromRequest<_, _>>::from_request(request, &|| {}).await;

        let outcome = outcome.unwrap();
        assert_eq!(input, outcome.0);
    }

    #[tokio::test]
    async fn serializes_raw() {
        let input = Input { foo: "bar".into() };
        let serialized = rmp_serde::encode::to_vec(&input);
        assert!(serialized.is_ok());
        let serialized = serialized.unwrap();

        let body = MsgPackRaw(input).into_response().into_body();
        let bytes = to_bytes(body).await;

        assert_eq!(serialized, bytes);
    }

    #[tokio::test]
    async fn deserializes_raw() {
        let input = Input { foo: "bar".into() };
        let mut request = into_request_raw(&input);

        request.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/msgpack"),
        );

        let outcome = <MsgPackRaw<Input> as FromRequest<_, _>>::from_request(request, &|| {}).await;

        let outcome = outcome.unwrap();
        assert_eq!(input, outcome.0);
    }

    #[tokio::test]
    async fn supported_content_type() {
        let input = Input { foo: "bar".into() };
        let mut request = into_request(&input);
        request.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/msgpack"),
        );

        let outcome = <MsgPack<Input> as FromRequest<_, _>>::from_request(request, &|| {}).await;
        assert!(outcome.is_ok());

        let mut request = into_request(&input);
        request.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/cloudevents+msgpack"),
        );

        let outcome = <MsgPack<Input> as FromRequest<_, _>>::from_request(request, &|| {}).await;
        assert!(outcome.is_ok());

        let mut request = into_request(&input);
        request.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/x-msgpack"),
        );

        let outcome = <MsgPack<Input> as FromRequest<_, _>>::from_request(request, &|| {}).await;
        assert!(outcome.is_ok());

        let request = into_request(&input);
        let outcome = <MsgPack<Input> as FromRequest<_, _>>::from_request(request, &|| {}).await;

        match outcome {
            Err(MsgPackRejection::MissingMsgPackContentType(_)) => {}
            other => unreachable!(
                "Expected missing MsgPack content type rejection, got: {:?}",
                other
            ),
        }
    }

    async fn to_bytes(body: Body) -> Vec<u8> {
        let mut buffer = Vec::new();
        let mut stream = body.into_data_stream();

        while let Some(bytes) = stream.next().await {
            buffer.extend(bytes.unwrap().into_iter());
        }

        buffer
    }
}

// use crate::{
//     codec::{Patch, Post, Put},
//     ContentType, Decodes, Encodes, Format, FormatType,
// };
// use bytes::Bytes;
// use serde::{de::DeserializeOwned, Serialize};

// /// Serializes and deserializes MessagePack with [`rmp_serde`].
// pub struct MsgPackEncoding;

// impl ContentType for MsgPackEncoding {
//     const CONTENT_TYPE: &'static str = "application/msgpack";
// }

// impl FormatType for MsgPackEncoding {
//     const FORMAT_TYPE: Format = Format::Binary;
// }

// impl<T> Encodes<T> for MsgPackEncoding
// where
//     T: Serialize,
// {
//     type Error = rmp_serde::encode::Error;

//     fn encode(value: &T) -> Result<Bytes, Self::Error> {
//         rmp_serde::to_vec(value).map(Bytes::from)
//     }
// }

// impl<T> Decodes<T> for MsgPackEncoding
// where
//     T: DeserializeOwned,
// {
//     type Error = rmp_serde::decode::Error;

//     fn decode(bytes: Bytes) -> Result<T, Self::Error> {
//         rmp_serde::from_slice(&bytes)
//     }
// }

// /// Pass arguments and receive responses as MessagePack in a `POST` request.
// pub type MsgPack = Post<MsgPackEncoding>;

// /// Pass arguments and receive responses as MessagePack in the body of a `PATCH` request.
// /// **Note**: Browser support for `PATCH` requests without JS/WASM may be poor.
// /// Consider using a `POST` request if functionality without JS/WASM is required.
// pub type PatchMsgPack = Patch<MsgPackEncoding>;

// /// Pass arguments and receive responses as MessagePack in the body of a `PUT` request.
// /// **Note**: Browser support for `PUT` requests without JS/WASM may be poor.
// /// Consider using a `POST` request if functionality without JS/WASM is required.
// pub type PutMsgPack = Put<MsgPackEncoding>;
