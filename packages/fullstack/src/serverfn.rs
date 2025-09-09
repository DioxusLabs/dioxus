use base64::{engine::general_purpose::STANDARD_NO_PAD, DecodeError, Engine};

use crate::{
    ContentType,
    ContextProviders,
    Decodes,
    DioxusServerContext,
    Encodes,
    FormatType,
    FromServerFnError,
    ProvideServerContext,
    ServerFnError,
    // FromServerFnError, Protocol, ProvideServerContext, ServerFnError,
};

// use super::client::Client;
use super::codec::Encoding;
// use super::codec::{Encoding, FromReq, FromRes, IntoReq, IntoRes};

// #[cfg(feature = "form-redirects")]
// use super::error::ServerFnUrlError;

use super::middleware::{BoxedService, Layer, Service};
use super::redirect::call_redirect_hook;
// use super::response::{Res, TryRes};
// use super::response::{ClientRes, Res, TryRes};
use bytes::{BufMut, Bytes, BytesMut};
use dashmap::DashMap;
use futures::{pin_mut, SinkExt, Stream, StreamExt};
use http::{method, Method};

// use super::server::Server;
use std::{
    fmt::{Debug, Display},
    future::Future,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    pin::Pin,
    sync::{Arc, LazyLock},
};

type Req = HybridRequest;
type Res = HybridResponse;

/// A function endpoint that can be called from the client.
#[derive(Clone)]
pub struct ServerFunction {
    path: &'static str,
    method: Method,
    handler: fn(Req) -> Pin<Box<dyn Future<Output = Res> + Send>>,
    pub(crate) middleware: fn() -> MiddlewareSet<Req, Res>,
    ser: fn(ServerFnError) -> Bytes,
}

impl ServerFunction {
    /// Create a new server function object.
    pub const fn new(
        method: Method,
        path: &'static str,
        handler: fn(Req) -> Pin<Box<dyn Future<Output = Res> + Send>>,
        middlewares: Option<fn() -> MiddlewareSet<Req, Res>>,
    ) -> Self {
        fn default_middlewares<Req, Res>() -> MiddlewareSet<Req, Res> {
            Vec::new()
        }

        Self {
            path,
            method,
            handler,
            ser: |e| HybridError::from_server_fn_error(e).ser(),
            middleware: match middlewares {
                Some(m) => m,
                None => default_middlewares,
            },
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
    pub fn handler(&self, req: Req) -> impl Future<Output = Res> + Send {
        (self.handler)(req)
    }

    /// The set of middleware that should be applied to this function.
    pub fn middleware(&self) -> MiddlewareSet<Req, Res> {
        (self.middleware)()
    }

    /// Converts the server function into a boxed service.
    pub fn boxed(self) -> BoxedService<Req, Res>
    where
        Self: Service<Req, Res>,
        Req: 'static,
        Res: 'static,
    {
        BoxedService::new(self.ser, self)
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

        // this is taken from server_fn source...
        //
        // [`server_fn::axum::get_server_fn_service`]
        let mut service = {
            let middleware = self.middleware();
            let mut service = self.clone().boxed();
            for middleware in middleware {
                service = middleware.layer(service);
            }
            service
        };

        let req = crate::HybridRequest { req };

        // actually run the server fn (which may use the server context)
        let fut = crate::with_server_context(server_context.clone(), || service.run(req));

        let res = ProvideServerContext::new(fut, server_context.clone()).await;
        let mut res = res.res;

        // it it accepts text/html (i.e., is a plain form post) and doesn't already have a
        // Location set, then redirect to Referer
        if accepts_html {
            if let Some(referrer) = referrer {
                let has_location = res.headers().get(LOCATION).is_some();
                if !has_location {
                    *res.status_mut() = StatusCode::FOUND;
                    res.headers_mut().insert(LOCATION, referrer);
                }
            }
        }

        // apply the response parts from the server context to the response
        server_context.send_response(&mut res);

        res
    }

    pub(crate) fn collect_static() -> Vec<&'static ServerFunction> {
        inventory::iter::<ServerFunction>().collect()
    }
}

impl Service<Req, Res> for ServerFunction
where
    Req: Send + 'static,
    Res: 'static,
{
    fn run(
        &mut self,
        req: Req,
        _ser: fn(ServerFnError) -> Bytes,
    ) -> Pin<Box<dyn Future<Output = Res> + Send>> {
        let handler = self.handler;
        Box::pin(async move { handler(req).await })
    }
}

impl inventory::Collect for ServerFunction {
    #[inline]
    fn registry() -> &'static inventory::Registry {
        static REGISTRY: inventory::Registry = inventory::Registry::new();
        &REGISTRY
    }
}

pub struct HybridRequest {
    pub(crate) req: http::Request<axum::body::Body>,
}

pub struct HybridResponse {
    pub(crate) res: http::Response<axum::body::Body>,
}
pub struct HybridStreamError {}
pub type HybridError = ServerFnError;

/// A list of middlewares that can be applied to a server function.
pub type MiddlewareSet<Req, Res> = Vec<Arc<dyn Layer<Req, Res>>>;
