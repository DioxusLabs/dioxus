use std::prelude::rust_2024::Future;

pub use axum::extract::Json;
use serde::{de::DeserializeOwned, Serialize};

use crate::{FromResponse, ServerFnError};

use super::IntoRequest;

impl<T> IntoRequest<()> for Json<T>
where
    T: Serialize,
{
    type Input = T;
    type Output = Json<T>;

    fn into_request(input: Self::Input) -> Result<axum::Json<T>, ServerFnError> {
        Ok(Json(input))
    }
}

impl<T> FromResponse<()> for Json<T>
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
            Ok(Json(res))
        }
    }
}
