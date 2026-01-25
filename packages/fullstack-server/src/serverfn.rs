use crate::FullstackState;
use axum::{
    body::Body,
    extract::{Request, State},
    response::Response,
    routing::MethodRouter,
};
use dioxus_fullstack_core::FullstackContext;
use http::{Method, StatusCode};
use std::{pin::Pin, prelude::rust_2024::Future};

/// A function endpoint that can be called from the client.
#[derive(Clone)]
pub struct ServerFunction {
    path: &'static str,
    method: Method,
    handler: fn() -> MethodRouter<FullstackState>,
}

impl ServerFunction {
    /// Create a new server function object from a MethodRouter
    pub const fn new(
        method: Method,
        path: &'static str,
        handler: fn() -> MethodRouter<FullstackState>,
    ) -> Self {
        Self {
            path,
            method,
            handler,
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

    /// Collect all globally registered server functions
    pub fn collect() -> Vec<&'static ServerFunction> {
        inventory::iter::<ServerFunction>().collect()
    }

    /// Create a `MethodRouter` for this server function that can be mounted on an `axum::Router`.
    ///
    /// This runs the handler inside the required `FullstackContext` scope and populates
    /// `FullstackContext` so that the handler can use those features.
    ///
    /// It also runs the server function inside a tokio `LocalPool` to allow !Send futures.
    pub fn method_router(&self) -> MethodRouter<FullstackState> {
        (self.handler)()
    }

    /// Creates a new `MethodRouter` for the given method and !Send handler.
    ///
    /// This is used internally by the `ServerFunction` to create the method router that this
    /// server function uses.
    #[allow(clippy::type_complexity)]
    pub fn make_handler(
        method: Method,
        handler: fn(State<FullstackContext>, Request) -> Pin<Box<dyn Future<Output = Response>>>,
    ) -> MethodRouter<FullstackState> {
        axum::routing::method_routing::on(
            method
                .try_into()
                .expect("MethodFilter only supports standard HTTP methods"),
            move |state: State<FullstackState>, request: Request| async move {
                use tracing::Instrument;
                let current_span = tracing::Span::current();
                // Allow !Send futures by running in the render handlers pinned local pool
                let result = state.rt.spawn_pinned(move || async move {
                    use dioxus_fullstack_core::FullstackContext;
                    use http::header::{ACCEPT, LOCATION, REFERER};
                    use http::StatusCode;

                    // todo: we're copying the parts here, but it'd be ideal if we didn't.
                    // We can probably just pass the URI in so the matching logic can work and then
                    // in the server function, do all extraction via FullstackContext. This ensures
                    // calls to `.remove()` work as expected.
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
                        .clone()
                        .scope(async move {
                            // Run the next middleware / handler inside the server context
                            let mut response = handler(State(server_context), request)
                                .instrument(current_span)
                                .await;

                            let server_context = FullstackContext::current().expect(
                                "Server context should be available inside the server context scope",
                            );

                            // Get the extra response headers set during the handler and add them to the response
                            let headers = server_context.take_response_headers();
                            if let Some(headers) = headers {
                                response.headers_mut().extend(headers);
                            }

                            // if it accepts text/html (i.e., is a plain form post) and doesn't already have a
                            // Location set, then redirect to Referer
                            if accepts_html {
                                if let Some(referrer) = referrer {
                                    let has_location = response.headers().get(LOCATION).is_some();
                                    if !has_location {
                                        *response.status_mut() = StatusCode::FOUND;
                                        response.headers_mut().insert(LOCATION, referrer);
                                    }
                                }
                            }

                            response
                        })
                        .await
                }).await;

                match result {
                    Ok(response) => response,
                    Err(err) => Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::new(if cfg!(debug_assertions) {
                            format!("Server function panicked: {}", err)
                        } else {
                            "Internal Server Error".to_string()
                        }))
                        .unwrap(),
                }
            },
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
