//! server-fn codec just use the axum extractors
//! ie Json<T>, Form<T>, etc
//!
//! Axum gives us:
//! - Json<T>
//! - Form<T>
//! - Multipart<T>
//!
//! We need to build/copy:
//! - Cbor<T>
//! - MsgPack<T>
//! - Postcard<T>
//! - Rkyv<T>
//!
//! Others??
//! - url-encoded GET params?
//! - stream?

use std::prelude::rust_2024::Future;

use axum::extract::Form; // both req/res
use axum::extract::Json; // both req/res
use axum::extract::Multipart; // req only

pub mod cbor;
pub mod form;
pub mod json;
pub mod msgpack;
pub mod multipart;
pub mod postcard;
pub mod rkyv;

pub mod redirect;

pub mod sse;
pub use sse::*;

pub mod textstream;
pub use textstream::*;

pub mod websocket;
pub use websocket::*;

pub mod upload;
pub use upload::*;

pub mod req_from;
pub use req_from::*;

pub mod req_to;
pub use req_to::*;

#[macro_use]
/// Error types and utilities.
pub mod error;
pub use error::*;

/// Implementations of the client side of the server function call.
pub mod client;
pub use client::*;

pub trait FromResponse<M> {
    type Output;
    fn from_response(
        res: reqwest::Response,
    ) -> impl Future<Output = Result<Self::Output, ServerFnError>> + Send;
}

pub trait IntoRequest<M> {
    type Input;
    type Output;
    fn into_request(input: Self::Input) -> Result<Self::Output, ServerFnError>;
}

use axum::{extract::State, routing::MethodRouter, Router};
use base64::{engine::general_purpose::STANDARD_NO_PAD, DecodeError, Engine};
use dioxus_core::{Element, VirtualDom};

use crate::{
    ContextProviders,
    DioxusServerContext,
    ProvideServerContext,
    ServerFnError,
    // FromServerFnError, Protocol, ProvideServerContext, ServerFnError,
};
// use super::client::Client;
// use super::codec::Encoding;
// use super::codec::{Encoding, FromReq, FromRes, IntoReq, IntoRes};
// #[cfg(feature = "form-redirects")]
// use super::error::ServerFnUrlError;
// use super::middleware::{BoxedService, Layer, Service};
// use super::response::{Res, TryRes};
// use super::response::{ClientRes, Res, TryRes};
// use super::server::Server;
use super::redirect::call_redirect_hook;
use bytes::{BufMut, Bytes, BytesMut};
use dashmap::DashMap;
use futures::{pin_mut, SinkExt, Stream, StreamExt};
use http::{method, Method};
use std::{
    fmt::{Debug, Display},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    pin::Pin,
    sync::{Arc, LazyLock},
};

pub type AxumRequest = http::Request<axum::body::Body>;
pub type AxumResponse = http::Response<axum::body::Body>;

#[derive(Clone, Default)]
pub struct DioxusServerState {}

/// A function endpoint that can be called from the client.
#[derive(Clone)]
pub struct ServerFunction {
    path: &'static str,
    method: Method,
    handler: fn() -> MethodRouter<DioxusServerState>,
    // serde_err: fn(ServerFnError) -> Bytes,
    // pub(crate) middleware: fn() -> MiddlewareSet<Req, Res>,
}

impl ServerFunction {
    /// Create a new server function object.
    pub const fn new(
        method: Method,
        path: &'static str,
        handler: fn() -> MethodRouter<DioxusServerState>,
        // middlewares: Option<fn() -> MiddlewareSet<Req, Res>>,
    ) -> Self {
        // fn default_middlewares<Req, Res>() -> MiddlewareSet<Req, Res> {
        //     Vec::new()
        // }

        Self {
            path,
            method,
            handler,
            // serde_err: |e| todo!(),
            // serde_err: |e| ServerFnError::from_server_fn_error(e).ser(),
            // middleware: match middlewares {
            //     Some(m) => m,
            //     None => default_middlewares,
            // },
        }
    }

    /// The path of the server function.
    pub fn path(&self) -> &'static str {
        self.path
    }

    /// The HTTP method the server function expects.
    pub fn method(&self) -> Method {
        self.method.clone()
    }

    /// The handler for this server function.
    pub fn handler(&self) -> fn() -> MethodRouter<DioxusServerState> {
        self.handler
    }

    // /// The set of middleware that should be applied to this function.
    // pub fn middleware(&self) -> Vec {
    //     (self.middleware)()
    // }

    /// Create a router with all registered server functions and the render handler at `/` (basepath).
    ///
    ///
    pub fn into_router(dioxus_app: fn() -> Element) -> Router {
        let router = Router::new();

        // add middleware if any exist

        let mut router = Self::with_router(router);

        // Now serve the app at `/`
        router = router.fallback(axum::routing::get(
            move |state: State<DioxusServerState>| async move {
                let mut vdom = VirtualDom::new(dioxus_app);
                vdom.rebuild_in_place();
                axum::response::Html(dioxus_ssr::render(&vdom))
            },
        ));

        router.with_state(DioxusServerState {})
    }

    pub fn with_router(mut router: Router<DioxusServerState>) -> Router<DioxusServerState> {
        for server_fn in crate::inventory::iter::<ServerFunction>() {
            let method_router = (server_fn.handler)();
            router = router.route(server_fn.path(), method_router);
        }

        router
    }

    pub fn collect() -> Vec<&'static ServerFunction> {
        inventory::iter::<ServerFunction>().collect()
    }

    pub fn register_server_fn_on_router<S>(
        &'static self,
        router: axum::Router<S>,
        context_providers: ContextProviders,
    ) -> axum::Router<S>
    where
        S: Send + Sync + Clone + 'static,
    {
        use http::method::Method;
        let path = self.path();
        let method = self.method();
        let handler = move |req| self.handle_server_fns_inner(context_providers, req);
        match method {
            Method::GET => router.route(path, axum::routing::get(handler)),
            Method::POST => router.route(path, axum::routing::post(handler)),
            Method::PUT => router.route(path, axum::routing::put(handler)),
            _ => unimplemented!("Unsupported server function method: {}", method),
        }
    }

    pub async fn handle_server_fns_inner(
        &self,
        additional_context: ContextProviders,
        req: http::Request<axum::body::Body>,
    ) -> http::Response<axum::body::Body> {
        use axum::body;
        use axum::extract::State;
        use axum::routing::*;
        use axum::{
            body::Body,
            http::{Request, Response, StatusCode},
            response::IntoResponse,
        };
        use http::header::*;

        let (parts, body) = req.into_parts();
        let req = Request::from_parts(parts.clone(), body);

        // Create the server context with info from the request
        let server_context = DioxusServerContext::new(parts);

        // Provide additional context from the render state
        server_context.add_server_context(&additional_context);

        // store Accepts and Referrer in case we need them for redirect (below)
        let referrer = req.headers().get(REFERER).cloned();
        let accepts_html = req
            .headers()
            .get(ACCEPT)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.contains("text/html"))
            .unwrap_or(false);

        // // this is taken from server_fn source...
        // //
        // // [`server_fn::axum::get_server_fn_service`]
        // let mut service = {
        //     let middleware = self.middleware();
        //     let mut service = self.clone().boxed();
        //     for middleware in middleware {
        //         service = middleware.layer(service);
        //     }
        //     service
        // };

        // // actually run the server fn (which may use the server context)
        // let fut = crate::with_server_context(server_context.clone(), || service.run(req));

        // let res = ProvideServerContext::new(fut, server_context.clone()).await;
        // let mut res = res.res;

        // // it it accepts text/html (i.e., is a plain form post) and doesn't already have a
        // // Location set, then redirect to Referer
        // if accepts_html {
        //     if let Some(referrer) = referrer {
        //         let has_location = res.headers().get(LOCATION).is_some();
        //         if !has_location {
        //             *res.status_mut() = StatusCode::FOUND;
        //             res.headers_mut().insert(LOCATION, referrer);
        //         }
        //     }
        // }

        // // apply the response parts from the server context to the response
        // server_context.send_response(&mut res);

        // res

        todo!()
    }
}

impl inventory::Collect for ServerFunction {
    #[inline]
    fn registry() -> &'static inventory::Registry {
        static REGISTRY: inventory::Registry = inventory::Registry::new();
        &REGISTRY
    }
}

/// The set of all registered server function paths.
pub fn server_fn_paths() -> impl Iterator<Item = (&'static str, Method)> {
    REGISTERED_SERVER_FUNCTIONS
        .iter()
        .map(|item| (item.path(), item.method()))
}

type LazyServerFnMap = LazyLock<DashMap<(String, Method), ServerFunction>>;
static REGISTERED_SERVER_FUNCTIONS: LazyServerFnMap = std::sync::LazyLock::new(|| {
    crate::inventory::iter::<ServerFunction>
        .into_iter()
        .map(|obj| ((obj.path().to_string(), obj.method()), obj.clone()))
        .collect()
});

// /// An Axum handler that responds to a server function request.
// pub async fn handle_server_fn(req: HybridRequest) -> HybridResponse {
//     let path = req.uri().path();

//     if let Some(mut service) = get_server_fn_service(path, req.req.method().clone()) {
//         service.run(req).await
//     } else {
//         let res = Response::builder()
//             .status(StatusCode::BAD_REQUEST)
//             .body(Body::from(format!(
//                 "Could not find a server function at the route {path}. \
//                      \n\nIt's likely that either\n 1. The API prefix you \
//                      specify in the `#[server]` macro doesn't match the \
//                      prefix at which your server function handler is mounted, \
//                      or \n2. You are on a platform that doesn't support \
//                      automatic server function registration and you need to \
//                      call ServerFn::register_explicit() on the server \
//                      function type, somewhere in your `main` function.",
//             )))
//             .unwrap();

//         HybridResponse { res }
//     }
// }

// /// Returns the server function at the given path as a service that can be modified.
// fn get_server_fn_service(
//     path: &str,
//     method: Method,
// ) -> Option<BoxedService<HybridRequest, HybridResponse>> {
//     let key = (path.into(), method);
//     REGISTERED_SERVER_FUNCTIONS.get(&key).map(|server_fn| {
//         let middleware = (server_fn.middleware)();
//         let mut service = server_fn.clone().boxed();
//         for middleware in middleware {
//             service = middleware.layer(service);
//         }
//         service
//     })
// }

// /// Explicitly register a server function. This is only necessary if you are
// /// running the server in a WASM environment (or a rare environment that the
// /// `inventory` crate won't work in.).
// pub fn register_explicit<T>()
// where
//     T: ServerFn + 'static,
// {
//     REGISTERED_SERVER_FUNCTIONS.insert(
//         (T::PATH.into(), T::METHOD),
//         ServerFnTraitObj::new(T::METHOD, T::PATH, |req| Box::pin(T::run_on_server(req))),
//         // ServerFnTraitObj::new::<T>(|req| Box::pin(T::run_on_server(req))),
//     );
// }
