use axum::response::IntoResponse;
use dioxus_fullstack_core::ServerFnError;
use std::prelude::rust_2024::Future;

use crate::FromResponse;

pub struct OpaqueResponse {
    #[cfg(feature = "server")]
    inner: Option<axum::response::Response>,
}

#[cfg(feature = "server")]
impl OpaqueResponse {
    /// Create a new `OpaqueResponse` from any type that implements `IntoResponse`.
    pub fn new(inner: impl IntoResponse) -> Self {
        Self {
            inner: Some(inner.into_response()),
        }
    }
}

impl IntoResponse for OpaqueResponse {
    fn into_response(self) -> axum::response::Response {
        #[cfg(feature = "server")]
        {
            self.inner
                .expect("OpaqueResponse can only be converted into a response on the server")
        }
        #[cfg(not(feature = "server"))]
        {
            todo!()
        }
    }
}

impl FromResponse for OpaqueResponse {
    fn from_response(
        res: reqwest::Response,
    ) -> impl Future<Output = Result<Self, ServerFnError>> + Send {
        async move {
            Ok(OpaqueResponse {
                #[cfg(feature = "server")]
                inner: None,
            })
        }
    }
}
