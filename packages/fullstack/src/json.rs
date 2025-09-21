use std::prelude::rust_2024::Future;

pub use axum::extract::Json;
use serde::{de::DeserializeOwned, Serialize};

use crate::{FromResponse, ServerFnError};

use super::IntoRequest;

impl<T> IntoRequest for Json<T>
where
    T: Serialize + 'static,
{
    fn into_request(
        input: Self,
        request_builder: reqwest::RequestBuilder,
    ) -> impl Future<Output = Result<reqwest::Response, reqwest::Error>> + Send + 'static {
        send_wrapper::SendWrapper::new(async move {
            request_builder
                .header("Content-Type", "application/json")
                .json(&input.0)
                .send()
                .await
        })
    }
}

impl<T: DeserializeOwned> FromResponse for Json<T> {
    fn from_response(
        res: reqwest::Response,
    ) -> impl Future<Output = Result<Self, ServerFnError>> + Send {
        send_wrapper::SendWrapper::new(async move {
            let data = res
                .json::<T>()
                .await
                .map_err(|e| ServerFnError::Deserialization(e.to_string()))?;
            Ok(Json(data))
        })
    }
}

// use super::{Patch, Post, Put};
// use crate::{ContentType, Decodes, Encodes, Format, FormatType};
// use bytes::Bytes;
// use serde::{de::DeserializeOwned, Serialize};

// /// Serializes and deserializes JSON with [`serde_json`].
// pub struct JsonEncoding;

// impl ContentType for JsonEncoding {
//     const CONTENT_TYPE: &'static str = "application/json";
// }

// impl FormatType for JsonEncoding {
//     const FORMAT_TYPE: Format = Format::Text;
// }

// impl<T: Serialize> Encodes<T> for JsonEncoding {
//     type Error = serde_json::Error;

//     fn encode(output: &T) -> Result<Bytes, Self::Error> {
//         serde_json::to_vec(output).map(Bytes::from)
//     }
// }

// impl<T: DeserializeOwned> Decodes<T> for JsonEncoding {
//     type Error = serde_json::Error;

//     fn decode(bytes: Bytes) -> Result<T, Self::Error> {
//         serde_json::from_slice(&bytes)
//     }
// }

// // /// Pass arguments and receive responses as JSON in the body of a `POST` request.
// // pub type Json = Post<JsonEncoding>;

// // /// Pass arguments and receive responses as JSON in the body of a `PATCH` request.
// // /// **Note**: Browser support for `PATCH` requests without JS/WASM may be poor.
// // /// Consider using a `POST` request if functionality without JS/WASM is required.
// // pub type PatchJson = Patch<JsonEncoding>;

// // /// Pass arguments and receive responses as JSON in the body of a `PUT` request.
// // /// **Note**: Browser support for `PUT` requests without JS/WASM may be poor.
// // /// Consider using a `POST` request if functionality without JS/WASM is required.
// // pub type PutJson = Put<JsonEncoding>;
