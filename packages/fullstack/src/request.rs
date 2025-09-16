use std::prelude::rust_2024::Future;

use dioxus_fullstack_core::ServerFnError;
use serde::de::DeserializeOwned;

pub trait FromResponse<M>: Sized {
    fn from_response(
        res: reqwest::Response,
    ) -> impl Future<Output = Result<Self, ServerFnError>> + Send;
}

pub struct DefaultEncoding;
impl<T> FromResponse<DefaultEncoding> for T
where
    T: DeserializeOwned + 'static,
{
    fn from_response(
        res: reqwest::Response,
    ) -> impl Future<Output = Result<Self, ServerFnError>> + Send {
        async move {
            let res = res
                .json::<T>()
                .await
                .map_err(|e| ServerFnError::Deserialization(e.to_string()))?;
            Ok(res)
        }
    }
}

pub trait IntoRequest<M> {
    type Input;
    type Output;
    fn into_request(input: Self::Input) -> Result<Self::Output, ServerFnError>;
}
