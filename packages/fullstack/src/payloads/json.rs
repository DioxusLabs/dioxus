use crate::{FromResponse, IntoRequest, ServerFnError};
pub use axum::extract::Json;
use reqwest::RequestBuilder;
use serde::{de::DeserializeOwned, Serialize};
use std::prelude::rust_2024::Future;

impl<T> IntoRequest for Json<T>
where
    T: Serialize + 'static,
{
    fn into_request(
        self,
        request_builder: RequestBuilder,
    ) -> impl Future<Output = Result<reqwest::Response, reqwest::Error>> + Send + 'static {
        send_wrapper::SendWrapper::new(async move {
            request_builder
                .header("Content-Type", "application/json")
                .json(&self.0)
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
