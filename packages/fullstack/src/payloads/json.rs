use crate::{ClientRequest, ClientResponse, FromResponse, IntoRequest, ServerFnError};
pub use axum::extract::Json;
use dioxus_fullstack_core::RequestError;
use serde::{de::DeserializeOwned, Serialize};
use std::future::Future;

impl<T> IntoRequest for Json<T>
where
    T: Serialize + 'static,
{
    fn into_request(
        self,
        request: ClientRequest,
    ) -> impl Future<Output = Result<ClientResponse, RequestError>> + Send + 'static {
        send_wrapper::SendWrapper::new(async move {
            request
                .header("Content-Type", "application/json")
                .json(&self.0)
                .send()
                .await
        })
    }
}

impl<T: DeserializeOwned> FromResponse for Json<T> {
    fn from_response(
        res: ClientResponse,
    ) -> impl Future<Output = Result<Self, ServerFnError>> + Send {
        send_wrapper::SendWrapper::new(async move {
            let data = res.json::<T>().await?;
            Ok(Json(data))
        })
    }
}
