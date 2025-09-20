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

use axum::{body::Body, extract::Form};
use axum::{extract::Json, Router};
use axum::{extract::Multipart, handler::Handler}; // both req/res // both req/res // req only

use axum::{extract::State, routing::MethodRouter};
use base64::{engine::general_purpose::STANDARD_NO_PAD, DecodeError, Engine};
use dioxus_core::{Element, VirtualDom};
use dioxus_fullstack_core::DioxusServerState;
use serde::de::DeserializeOwned;

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

// pub fn get_client() -> &'static reqwest::Client {
//     static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| reqwest::Client::new());
//     &CLIENT
// }

pub type AxumRequest = http::Request<Body>;
pub type AxumResponse = http::Response<Body>;

/// A function endpoint that can be called from the client.
#[derive(Clone)]
pub struct ServerFunction<Caller = ()> {
    path: &'static str,
    method: Method,
    handler: fn() -> MethodRouter<DioxusServerState>,
    _phantom: PhantomData<Caller>,
}

impl<In1, In2, Out> std::ops::Deref for ServerFunction<((In1, In2), Out)> {
    type Target = fn(In1, In2) -> MakeRequest<Out>;

    fn deref(&self) -> &Self::Target {
        todo!()
    }
}

pub struct MakeRequest<T> {
    _phantom: PhantomData<T>,
}

impl ServerFunction {
    /// Create a new server function object.
    pub const fn new(
        method: Method,
        path: &'static str,
        handler: fn() -> MethodRouter<DioxusServerState>,
    ) -> Self {
        Self {
            path,
            method,
            handler,
            _phantom: PhantomData,
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

    pub fn collect() -> Vec<&'static ServerFunction> {
        inventory::iter::<ServerFunction>().collect()
    }

    pub fn register_server_fn_on_router<S>(
        &'static self,
        router: Router<S>,
        context_providers: ContextProviders,
    ) -> Router<S>
    where
        S: Send + Sync + Clone + 'static,
    {
        tracing::info!(
            "Registering server function: {} {}",
            self.method(),
            self.path()
        );
        use http::method::Method;
        let path = self.path();
        let method = self.method();
        let handler = move |req| self.handle_server_fns_inner(context_providers, req);
        match method {
            Method::GET => router.route(path, axum::routing::get(handler)),
            Method::POST => router.route(path, axum::routing::post(handler)),
            Method::PUT => router.route(path, axum::routing::put(handler)),
            Method::DELETE => router.route(path, axum::routing::delete(handler)),
            Method::PATCH => router.route(path, axum::routing::patch(handler)),
            Method::HEAD => router.route(path, axum::routing::head(handler)),
            Method::OPTIONS => router.route(path, axum::routing::options(handler)),
            Method::CONNECT => router.route(path, axum::routing::connect(handler)),
            Method::TRACE => router.route(path, axum::routing::trace(handler)),
            _ => unimplemented!("Unsupported server function method: {}", method),
        }
    }

    pub async fn handle_server_fns_inner(
        &self,
        additional_context: ContextProviders,
        req: http::Request<Body>,
    ) -> http::Response<Body> {
        use axum::body;
        use axum::extract::State;
        use axum::routing::*;
        use axum::{
            body::Body,
            http::{Request, Response, StatusCode},
            response::IntoResponse,
        };
        use http::header::*;

        // let (parts, body) = req.into_parts();
        // let req = Request::from_parts(parts.clone(), body);

        // // Create the server context with info from the request
        // let server_context = DioxusServerContext::new(parts);

        // // Provide additional context from the render state
        // server_context.add_server_context(&additional_context);

        // // store Accepts and Referrer in case we need them for redirect (below)
        // let referrer = req.headers().get(REFERER).cloned();
        // let accepts_html = req
        //     .headers()
        //     .get(ACCEPT)
        //     .and_then(|v| v.to_str().ok())
        //     .map(|v| v.contains("text/html"))
        //     .unwrap_or(false);

        // let mthd: MethodRouter<()> = axum::routing::get(self.handler);

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

        // actually run the server fn (which may use the server context)
        // let fut = crate::with_server_context(server_context.clone(), || service.run(req));
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

        tracing::info!(
            "Handling server function: {} {} with {} extensions",
            self.method(),
            self.path(),
            req.extensions().len()
        );

        let mthd: MethodRouter<DioxusServerState> =
            (self.handler)().with_state(DioxusServerState {});
        let res = mthd.call(req, DioxusServerState {}).await;

        res
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

pub struct EncodedServerFnRequest {
    pub path: String,
    pub method: Method,
}

// pub struct ServerFunction<Caller = ()> {
//     pub path: &'static str,
//     pub method: http::Method,
//     _p: std::marker::PhantomData<Caller>,
// }

// impl<T> Clone for ServerFunction<T> {
//     fn clone(&self) -> Self {
//         Self {
//             path: self.path,
//             method: self.method.clone(),
//             _p: std::marker::PhantomData,
//         }
//     }
// }

// impl ServerFunction {
//     pub const fn new<P>(method: http::Method, path: &'static str, handler: fn() -> P) -> Self {
//         Self {
//             path,
//             method,
//             _p: std::marker::PhantomData,
//         }
//     }

//     /// Get the full URL for this server function.
//     pub fn url(&self) -> String {
//         format!("{}{}", get_server_url(), self.path)
//     }
// }

// impl inventory::Collect for ServerFunction {
//     #[inline]
//     fn registry() -> &'static inventory::Registry {
//         static REGISTRY: inventory::Registry = inventory::Registry::new();
//         &REGISTRY
//     }
// }
