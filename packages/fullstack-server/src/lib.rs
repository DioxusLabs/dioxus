#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
// #![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unexpected_cfgs)]

// re-exported to make it possible to implement a custom Client without adding a separate
// dependency on `bytes`
pub use bytes::Bytes;
pub use dioxus_fullstack_core::{ServerFnError, ServerFnResult};

pub use axum;
pub use config::ServeConfig;
pub use config::*;
pub use document::ServerDocument;
pub use http;
pub use inventory;
pub use server::*;

pub mod redirect;

#[cfg(not(target_arch = "wasm32"))]
mod launch;

#[cfg(not(target_arch = "wasm32"))]
pub use launch::{launch, launch_cfg};

/// Implementations of the server side of the server function call.
pub mod server;

/// Types and traits for HTTP responses.
// pub mod response;
pub mod config;

pub(crate) mod document;
pub(crate) mod ssr;
pub(crate) mod streaming;

pub use launch::router;
pub use launch::serve;

pub mod serverfn;
pub use serverfn::*;

pub mod isrg;
pub use isrg::*;

mod index_html;
pub(crate) use index_html::IndexHtml;

pub mod extract {
    use axum::extract::FromRequestParts;
    use axum_core::__composite_rejection as composite_rejection;
    use axum_core::__define_rejection as define_rejection;
    use http::{request::Parts};
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
            let inner: T = serde_qs::from_str(&parts.uri.query().unwrap_or_default()).map_err(FailedToDeserializeQueryString::from_err)?;
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
}
