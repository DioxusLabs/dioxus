use axum::body::Body;
use axum::routing::MethodRouter;
use axum::Router;
use dashmap::DashMap;
use dioxus_fullstack_core::{DioxusServerState, FullstackContext};
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

    pub fn handler(&self) -> fn() -> MethodRouter<DioxusServerState> {
        self.handler
    }

    pub fn register_server_fn_on_router<S>(&'static self, router: Router<S>) -> Router<S>
    where
        S: Send + Sync + Clone + 'static,
    {
        use axum::{extract::Request, middleware::Next};

        router.route(
            self.path(),
            ((self.handler)())
                .with_state(DioxusServerState {})
                .layer(axum::middleware::from_fn(
                    |request: Request, next: Next| async move {
                        use http::header::{ACCEPT, LOCATION, REFERER};
                        use http::StatusCode;

                        let (parts, body) = request.into_parts();
                        let server_context = FullstackContext::new(parts.clone());
                        let request = axum::extract::Request::from_parts(parts, body);

                        // store Accepts and Referrer in case we need them for redirect (below)
                        let referrer = request.headers().get(REFERER).cloned();
                        let accepts_html = request
                            .headers()
                            .get(ACCEPT)
                            .and_then(|v| v.to_str().ok())
                            .map(|v| v.contains("text/html"))
                            .unwrap_or(false);

                        server_context
                            .scope(async move {
                                // Run the next middleware / handler inside the server context
                                let mut response = next.run(request).await;

                                let server_context = FullstackContext::current().expect("Server context should be available inside the server context scope",);

                                // Get the extra response headers set during the handler and add them to the response
                                let headers = server_context.take_response_headers();
                                if let Some(headers) = headers {
                                    response.headers_mut().extend(headers);
                                }

                                // it it accepts text/html (i.e., is a plain form post) and doesn't already have a
                                // Location set, then redirect to Referer
                                if accepts_html {
                                    if let Some(referrer) = referrer {
                                        let has_location =
                                            response.headers().get(LOCATION).is_some();
                                        if !has_location {
                                            *response.status_mut() = StatusCode::FOUND;
                                            response.headers_mut().insert(LOCATION, referrer);
                                        }
                                    }
                                }

                                response
                            })
                            .await
                    },
                )),
        )
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
