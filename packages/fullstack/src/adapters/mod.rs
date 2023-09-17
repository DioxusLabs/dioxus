//! # Adapters
//! Adapters for different web frameworks.
//!
//! Each adapter provides a set of utilities that is ergonomic to use with the framework.
//!
//! Each framework has utilies for some or all of the following:
//! - Server functions
//!  - A generic way to register server functions
//!  - A way to register server functions with a custom handler that allows users to pass in a custom [`crate::server_context::DioxusServerContext`] based on the state of the server framework.
//! - A way to register static WASM files that is accepts [`crate::serve_config::ServeConfig`]
//! - A hot reloading web socket that intigrates with [`dioxus-hot-reload`](https://crates.io/crates/dioxus-hot-reload)

#[cfg(feature = "axum")]
pub mod axum_adapter;
#[cfg(feature = "salvo")]
pub mod salvo_adapter;
#[cfg(feature = "warp")]
pub mod warp_adapter;

use http::StatusCode;
use server_fn::{Encoding, Payload};
use std::sync::{Arc, RwLock};

use crate::{
    layer::{BoxedService, Service},
    prelude::{DioxusServerContext, ProvideServerContext},
};

/// Create a server function handler with the given server context and server function.
pub fn server_fn_service(
    context: DioxusServerContext,
    function: server_fn::ServerFnTraitObj<()>,
) -> crate::layer::BoxedService {
    let prefix = function.prefix().to_string();
    let url = function.url().to_string();
    if let Some(middleware) = crate::server_fn::MIDDLEWARE.get(&(&prefix, &url)) {
        let mut service = BoxedService(Box::new(ServerFnHandler::new(context, function)));
        for middleware in middleware {
            service = middleware.layer(service);
        }
        service
    } else {
        BoxedService(Box::new(ServerFnHandler::new(context, function)))
    }
}

#[derive(Clone)]
/// A default handler for server functions. It will deserialize the request body, call the server function, and serialize the response.
pub struct ServerFnHandler {
    server_context: DioxusServerContext,
    function: server_fn::ServerFnTraitObj<()>,
}

impl ServerFnHandler {
    /// Create a new server function handler with the given server context and server function.
    pub fn new(
        server_context: impl Into<DioxusServerContext>,
        function: server_fn::ServerFnTraitObj<()>,
    ) -> Self {
        let server_context = server_context.into();
        Self {
            server_context,
            function,
        }
    }
}

impl Service for ServerFnHandler {
    fn run(
        &mut self,
        req: http::Request<hyper::body::Body>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<http::Response<hyper::body::Body>, server_fn::ServerFnError>,
                > + Send,
        >,
    > {
        let Self {
            server_context,
            function,
        } = self.clone();
        Box::pin(async move {
            let query = req.uri().query().unwrap_or_default().as_bytes().to_vec();
            let (parts, body) = req.into_parts();
            let body = hyper::body::to_bytes(body).await?.to_vec();
            let headers = &parts.headers;
            let accept_header = headers.get("Accept").cloned();
            let parts = Arc::new(RwLock::new(parts));

            // Because the future returned by `server_fn_handler` is `Send`, and the future returned by this function must be send, we need to spawn a new runtime
            let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
            let pool = get_local_pool();
            pool.spawn_pinned({
                let function = function.clone();
                let mut server_context = server_context.clone();
                server_context.parts = parts;
                move || async move {
                    let data = match function.encoding() {
                        Encoding::Url | Encoding::Cbor => &body,
                        Encoding::GetJSON | Encoding::GetCBOR => &query,
                    };
                    let server_function_future = function.call((), data);
                    let server_function_future =
                        ProvideServerContext::new(server_function_future, server_context.clone());
                    let resp = server_function_future.await;

                    resp_tx.send(resp).unwrap();
                }
            });
            let result = resp_rx.await.unwrap();
            let mut res = http::Response::builder();

            // Set the headers from the server context
            let parts = server_context.response_parts().unwrap();
            *res.headers_mut().expect("empty headers should be valid") = parts.headers.clone();

            let serialized = result?;
            // if this is Accept: application/json then send a serialized JSON response
            let accept_header = accept_header.as_ref().and_then(|value| value.to_str().ok());
            if accept_header == Some("application/json")
                || accept_header
                    == Some(
                        "application/\
                                x-www-form-urlencoded",
                    )
                || accept_header == Some("application/cbor")
            {
                res = res.status(StatusCode::OK);
            }

            Ok(match serialized {
                Payload::Binary(data) => {
                    res = res.header("Content-Type", "application/cbor");
                    res.body(data.into())?
                }
                Payload::Url(data) => {
                    res = res.header(
                        "Content-Type",
                        "application/\
                                    x-www-form-urlencoded",
                    );
                    res.body(data.into())?
                }
                Payload::Json(data) => {
                    res = res.header("Content-Type", "application/json");
                    res.body(data.into())?
                }
            })
        })
    }
}

fn get_local_pool() -> tokio_util::task::LocalPoolHandle {
    use once_cell::sync::OnceCell;
    static LOCAL_POOL: OnceCell<tokio_util::task::LocalPoolHandle> = OnceCell::new();
    LOCAL_POOL
        .get_or_init(|| {
            tokio_util::task::LocalPoolHandle::new(
                std::thread::available_parallelism()
                    .map(Into::into)
                    .unwrap_or(1),
            )
        })
        .clone()
}
