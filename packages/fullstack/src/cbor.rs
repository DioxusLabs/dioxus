// use super::{Patch, Post, Put};
// use crate::{ContentType, Decodes, Encodes, Format, FormatType};
// use bytes::Bytes;
// use serde::{de::DeserializeOwned, Serialize};

// /// Serializes and deserializes CBOR with [`ciborium`].
// pub struct CborEncoding;

// impl ContentType for CborEncoding {
//     const CONTENT_TYPE: &'static str = "application/cbor";
// }

// impl FormatType for CborEncoding {
//     const FORMAT_TYPE: Format = Format::Binary;
// }

// impl<T> Encodes<T> for CborEncoding
// where
//     T: Serialize,
// {
//     type Error = ciborium::ser::Error<std::io::Error>;

//     fn encode(value: &T) -> Result<Bytes, Self::Error> {
//         let mut buffer: Vec<u8> = Vec::new();
//         ciborium::ser::into_writer(value, &mut buffer)?;
//         Ok(Bytes::from(buffer))
//     }
// }

// impl<T> Decodes<T> for CborEncoding
// where
//     T: DeserializeOwned,
// {
//     type Error = ciborium::de::Error<std::io::Error>;

//     fn decode(bytes: Bytes) -> Result<T, Self::Error> {
//         ciborium::de::from_reader(bytes.as_ref())
//     }
// }

// /// Pass arguments and receive responses using `cbor` in a `POST` request.
// pub type Cbor = Post<CborEncoding>;

// /// Pass arguments and receive responses using `cbor` in the body of a `PATCH` request.
// /// **Note**: Browser support for `PATCH` requests without JS/WASM may be poor.
// /// Consider using a `POST` request if functionality without JS/WASM is required.
// pub type PatchCbor = Patch<CborEncoding>;

// /// Pass arguments and receive responses using `cbor` in the body of a `PUT` request.
// /// **Note**: Browser support for `PUT` requests without JS/WASM may be poor.
// /// Consider using a `POST` request if functionality without JS/WASM is required.
// pub type PutCbor = Put<CborEncoding>;

// use axum::extract::Json;
// use axum::extract::OptionalFromRequest;
// use axum::extract::{FromRequest, Request};
// use axum::response::{IntoResponse, Response};
// use bytes::{BufMut, Bytes, BytesMut};
// use http::{
//     header::{self, HeaderMap, HeaderValue},
//     StatusCode,
// };
// use serde::{de::DeserializeOwned, Serialize};

// pub struct Cbor<T>(pub T);

// #[derive(Debug)]
// pub struct CborRejection;

// impl IntoResponse for CborRejection {
//     fn into_response(self) -> Response {
//         (StatusCode::BAD_REQUEST, "Invalid CBOR").into_response()
//     }
// }

// impl<T, S> FromRequest<S> for Cbor<T>
// where
//     T: DeserializeOwned,
//     S: Send + Sync,
// {
//     type Rejection = CborRejection;

//     async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
//         if !cbor_content_type(req.headers()) {
//             return Err(CborRejection);
//         }

//         let bytes = Bytes::from_request(req, state)
//             .await
//             .map_err(|_| CborRejection)?;
//         Self::from_bytes(&bytes)
//     }
// }

// impl<T, S> OptionalFromRequest<S> for Cbor<T>
// where
//     T: DeserializeOwned,
//     S: Send + Sync,
// {
//     type Rejection = CborRejection;

//     async fn from_request(req: Request, state: &S) -> Result<Option<Self>, Self::Rejection> {
//         let headers = req.headers();
//         if headers.get(header::CONTENT_TYPE).is_some() {
//             if cbor_content_type(headers) {
//                 let bytes = Bytes::from_request(req, state)
//                     .await
//                     .map_err(|_| CborRejection)?;
//                 Ok(Some(Self::from_bytes(&bytes)?))
//             } else {
//                 Err(CborRejection)
//             }
//         } else {
//             Ok(None)
//         }
//     }
// }

// fn cbor_content_type(headers: &HeaderMap) -> bool {
//     let Some(content_type) = headers.get(header::CONTENT_TYPE) else {
//         return false;
//     };

//     let Ok(content_type) = content_type.to_str() else {
//         return false;
//     };

//     content_type == "application/cbor"
// }

// impl<T> From<T> for Cbor<T> {
//     fn from(inner: T) -> Self {
//         Self(inner)
//     }
// }

// impl<T> Cbor<T>
// where
//     T: DeserializeOwned,
// {
//     /// Construct a `Cbor<T>` from a byte slice.
//     pub fn from_bytes(bytes: &[u8]) -> Result<Self, CborRejection> {
//         match ciborium::de::from_reader(bytes) {
//             Ok(value) => Ok(Cbor(value)),
//             Err(_) => Err(CborRejection),
//         }
//     }
// }

// impl<T> IntoResponse for Cbor<T>
// where
//     T: Serialize,
// {
//     fn into_response(self) -> Response {
//         let mut buf = Vec::new();
//         match ciborium::ser::into_writer(&self.0, &mut buf) {
//             Ok(()) => (
//                 [(
//                     header::CONTENT_TYPE,
//                     HeaderValue::from_static("application/cbor"),
//                 )],
//                 buf,
//             )
//                 .into_response(),
//             Err(err) => (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 [(
//                     header::CONTENT_TYPE,
//                     HeaderValue::from_static("text/plain; charset=utf-8"),
//                 )],
//                 err.to_string(),
//             )
//                 .into_response(),
//         }
//     }
// }
