use axum::extract::FromRequestParts;
use axum_core::__composite_rejection as composite_rejection;
use axum_core::__define_rejection as define_rejection;
use http::request::Parts;
use serde_core::de::DeserializeOwned;

#[derive(Debug, Clone, Copy, Default)]
pub struct Query<T>(pub T);

impl<T, S> FromRequestParts<S> for Query<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = QueryRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let inner: T = serde_qs::from_str(parts.uri.query().unwrap_or_default())
            .map_err(FailedToDeserializeQueryString::from_err)?;
        Ok(Self(inner))
    }
}

axum_core::__impl_deref!(Query);

define_rejection! {
    #[status = BAD_REQUEST]
    #[body = "Failed to deserialize query string"]
    pub struct FailedToDeserializeQueryString(Error);
}

composite_rejection! {
    pub enum QueryRejection {
        FailedToDeserializeQueryString,
    }
}
