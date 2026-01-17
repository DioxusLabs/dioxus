use std::ops::Deref;

use crate::ServerFnError;
use axum::extract::FromRequestParts;
use http::request::Parts;
use serde::de::DeserializeOwned;

/// An extractor that deserializes query parameters into the given type `T`.
///
/// This uses `serde_qs` under the hood to support complex query parameter structures.
#[derive(Debug, Clone, Copy, Default)]
pub struct Query<T>(pub T);

impl<T, S> FromRequestParts<S> for Query<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ServerFnError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let inner: T = serde_qs::from_str(parts.uri.query().unwrap_or_default())
            .map_err(|e| ServerFnError::Deserialization(e.to_string()))?;
        Ok(Self(inner))
    }
}

impl<T> Deref for Query<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
