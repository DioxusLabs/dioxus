use axum::body::Body;
use axum::handler::Handler;
use axum::routing::MethodRouter;
use axum::Router; // both req/res // both req/res // req only
use dashmap::DashMap;
use dioxus_fullstack_core::DioxusServerState;
use http::Method;
use std::{marker::PhantomData, sync::LazyLock};

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

    pub fn register_server_fn_on_router<S>(&'static self, router: Router<S>) -> Router<S>
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
        let handler = move |req| self.handle_server_fns_inner(req);
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

    pub async fn handle_server_fns_inner(&self, req: http::Request<Body>) -> http::Response<Body> {
        // todo: jon
        // - bring back middleware to serverfns. the new layer system is fine but isn't a full replacement
        //
        // use axum::body;
        // use axum::extract::State;
        // use axum::routing::*;
        // use axum::{
        //     body::Body,
        //     http::{Request, Response, StatusCode},
        //     response::IntoResponse,
        // };
        // use http::header::*;

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

        let mthd: MethodRouter<DioxusServerState> =
            (self.handler)().with_state(DioxusServerState {});

        mthd.call(req, DioxusServerState {}).await
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
    inventory::iter::<ServerFunction>
        .into_iter()
        .map(|obj| ((obj.path().to_string(), obj.method()), obj.clone()))
        .collect()
});
