use crate::{render::SSRError, with_server_context, DioxusServerContext, SSRState, ServeConfig};
use crate::{ContextProviders, ProvideServerContext};
use axum::body;
use axum::extract::State;
use axum::routing::*;
use axum::{
    body::Body,
    http::{Request, Response, StatusCode},
    response::IntoResponse,
};
use dioxus_lib::prelude::{Element, VirtualDom};
use http::header::*;
use server_fn::ServerFnTraitObj;
use std::sync::Arc;

/// A extension trait with utilities for integrating Dioxus with your Axum router.
pub trait DioxusRouterExt<S>: DioxusRouterFnExt<S> {
    /// Serves the static WASM for your Dioxus application (except the generated index.html).
    ///
    /// # Example
    /// ```rust, no_run
    /// # #![allow(non_snake_case)]
    /// # use dioxus_lib::prelude::*;
    /// # use dioxus_fullstack::prelude::*;
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = dioxus::cli_config::fullstack_address_or_localhost();
    ///     let router = axum::Router::new()
    ///         // Server side render the application, serve static assets, and register server functions
    ///         .serve_static_assets()
    ///         // Server render the application
    ///         // ...
    ///         .into_make_service();
    ///     let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    ///     axum::serve(listener, router).await.unwrap();
    /// }
    /// ```
    fn serve_static_assets(self) -> Self
    where
        Self: Sized;

    /// Serves the Dioxus application. This will serve a complete server side rendered application.
    /// This will serve static assets, server render the application, register server functions, and integrate with hot reloading.
    ///
    /// # Example
    /// ```rust, no_run
    /// # #![allow(non_snake_case)]
    /// # use dioxus_lib::prelude::*;
    /// # use dioxus_fullstack::prelude::*;
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = dioxus::cli_config::fullstack_address_or_localhost();
    ///     let router = axum::Router::new()
    ///         // Server side render the application, serve static assets, and register server functions
    ///         .serve_dioxus_application(ServeConfig::new().unwrap(), app)
    ///         .into_make_service();
    ///     let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    ///     axum::serve(listener, router).await.unwrap();
    /// }
    ///
    /// fn app() -> Element {
    ///     rsx! { "Hello World" }
    /// }
    /// ```
    fn serve_dioxus_application(self, cfg: ServeConfig, app: fn() -> Element) -> Self
    where
        Self: Sized;
}

#[cfg(not(target_arch = "wasm32"))]
impl<S> DioxusRouterExt<S> for Router<S>
where
    S: Send + Sync + Clone + 'static,
{
    fn serve_static_assets(mut self) -> Self {
        use tower_http::services::{ServeDir, ServeFile};

        let public_path = crate::public_path();

        if !public_path.exists() {
            return self;
        }

        // Serve all files in public folder except index.html
        let dir = std::fs::read_dir(&public_path).unwrap_or_else(|e| {
            panic!(
                "Couldn't read public directory at {:?}: {}",
                &public_path, e
            )
        });

        for entry in dir.flatten() {
            let path = entry.path();
            if path.ends_with("index.html") {
                continue;
            }
            let route = path
                .strip_prefix(&public_path)
                .unwrap()
                .iter()
                .map(|segment| {
                    segment.to_str().unwrap_or_else(|| {
                        panic!("Failed to convert path segment {:?} to string", segment)
                    })
                })
                .collect::<Vec<_>>()
                .join("/");
            let route = format!("/{}", route);
            if path.is_dir() {
                self = self.nest_service(&route, ServeDir::new(path).precompressed_br());
            } else {
                self = self.nest_service(&route, ServeFile::new(path).precompressed_br());
            }
        }

        self
    }

    fn serve_dioxus_application(self, cfg: ServeConfig, app: fn() -> Element) -> Self {
        // Add server functions and render index.html
        let server = self
            .serve_static_assets()
            .register_server_functions_with_context(cfg.context_providers.clone());

        let ssr_state = SSRState::new(&cfg);

        server.fallback(
            get(render_handler)
                .with_state(RenderHandleState::new(cfg, app).with_ssr_state(ssr_state)),
        )
    }
}

/// A extension trait with server function utilities for integrating Dioxus with your Axum router.
pub trait DioxusRouterFnExt<S> {
    /// Registers server functions with the default handler. This handler function will pass an empty [`DioxusServerContext`] to your server functions.
    ///
    /// # Example
    /// ```rust, no_run
    /// # use dioxus_lib::prelude::*;
    /// # use dioxus_fullstack::prelude::*;
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = dioxus::cli_config::fullstack_address_or_localhost();
    ///     let router = axum::Router::new()
    ///         // Register server functions routes with the default handler
    ///         .register_server_functions()
    ///         .into_make_service();
    ///     let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    ///     axum::serve(listener, router).await.unwrap();
    /// }
    /// ```
    #[allow(dead_code)]
    fn register_server_functions(self) -> Self
    where
        Self: Sized,
    {
        self.register_server_functions_with_context(Default::default())
    }

    /// Registers server functions with some additional context to insert into the [`DioxusServerContext`] for that handler.
    ///
    /// # Example
    /// ```rust, no_run
    /// # use dioxus_lib::prelude::*;
    /// # use dioxus_fullstack::prelude::*;
    /// # use std::sync::Arc;
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = dioxus::cli_config::fullstack_address_or_localhost();
    ///     let router = axum::Router::new()
    ///         // Register server functions routes with the default handler
    ///         .register_server_functions_with_context(Arc::new(vec![Box::new(|| Box::new(1234567890u32))]))
    ///         .into_make_service();
    ///     let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    ///     axum::serve(listener, router).await.unwrap();
    /// }
    /// ```
    fn register_server_functions_with_context(self, context_providers: ContextProviders) -> Self;

    /// Serves a Dioxus application without static assets.
    /// Sets up server function routes and rendering endpoints only.
    ///
    /// Useful for WebAssembly environments or when static assets
    /// are served by another system.
    ///
    /// # Example
    /// ```rust, no_run
    /// # use dioxus_lib::prelude::*;
    /// # use dioxus_fullstack::prelude::*;
    /// #[tokio::main]
    /// async fn main() {
    ///     let router = axum::Router::new()
    ///         .serve_api_application(ServeConfig::new().unwrap(), app)
    ///         .into_make_service();
    ///     // ...
    /// }
    fn serve_api_application(self, cfg: ServeConfig, app: fn() -> Element) -> Self
    where
        Self: Sized;
}

impl<S> DioxusRouterFnExt<S> for Router<S>
where
    S: Send + Sync + Clone + 'static,
{
    fn register_server_functions_with_context(
        mut self,
        context_providers: ContextProviders,
    ) -> Self {
        for f in collect_raw_server_fns() {
            self = register_server_fn_on_router(f, self, context_providers.clone());
        }
        self
    }

    fn serve_api_application(self, cfg: ServeConfig, app: fn() -> Element) -> Self
    where
        Self: Sized,
    {
        let server = self.register_server_functions_with_context(cfg.context_providers.clone());

        let ssr_state = SSRState::new(&cfg);

        server.fallback(
            get(render_handler)
                .with_state(RenderHandleState::new(cfg, app).with_ssr_state(ssr_state)),
        )
    }
}

pub fn register_server_fn_on_router<S>(
    f: &'static AxumServerFn,
    router: Router<S>,
    context_providers: ContextProviders,
) -> Router<S>
where
    S: Send + Sync + Clone + 'static,
{
    use http::method::Method;
    let path = f.path();
    let method = f.method();

    tracing::trace!("Registering server function: {} {}", method, path);
    let handler = move |req| handle_server_fns_inner(f, context_providers, req);
    match method {
        Method::GET => router.route(path, get(handler)),
        Method::POST => router.route(path, post(handler)),
        Method::PUT => router.route(path, put(handler)),
        _ => unimplemented!("Unsupported server function method: {}", method),
    }
}

pub type AxumServerFn = ServerFnTraitObj<http::Request<Body>, http::Response<Body>>;

pub(crate) fn collect_raw_server_fns() -> Vec<&'static AxumServerFn> {
    inventory::iter::<AxumServerFn>().collect()
}

/// A handler for Dioxus server functions. This will run the server function and return the result.
async fn handle_server_fns_inner(
    f: &AxumServerFn,
    additional_context: ContextProviders,
    req: Request<Body>,
) -> Response<axum::body::Body> {
    let (parts, body) = req.into_parts();
    let req = Request::from_parts(parts.clone(), body);

    // Create the server context with info from the request
    let server_context = DioxusServerContext::new(parts);

    // Provide additional context from the render state
    add_server_context(&server_context, &additional_context);

    // store Accepts and Referrer in case we need them for redirect (below)
    let accepts_html = req
        .headers()
        .get(ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("text/html"))
        .unwrap_or(false);
    let referrer = req.headers().get(REFERER).cloned();

    // this is taken from server_fn source...
    //
    // [`server_fn::axum::get_server_fn_service`]
    let mut service = {
        let middleware = f.middleware();
        let mut service = f.clone().boxed();
        for middleware in middleware {
            service = middleware.layer(service);
        }
        service
    };

    // actually run the server fn (which may use the server context)
    let fut = with_server_context(server_context.clone(), || service.run(req));

    let mut res = ProvideServerContext::new(fut, server_context.clone()).await;

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

pub(crate) fn add_server_context(
    server_context: &DioxusServerContext,
    context_providers: &ContextProviders,
) {
    for index in 0..context_providers.len() {
        let context_providers = context_providers.clone();
        server_context.insert_boxed_factory(Box::new(move || context_providers[index]()));
    }
}

/// State used by [`render_handler`] to render a dioxus component with axum
#[derive(Clone)]
pub struct RenderHandleState {
    config: ServeConfig,
    build_virtual_dom: Arc<dyn Fn() -> VirtualDom + Send + Sync>,
    ssr_state: std::sync::OnceLock<SSRState>,
}

impl RenderHandleState {
    /// Create a new [`RenderHandleState`]
    pub fn new(config: ServeConfig, root: fn() -> Element) -> Self {
        Self {
            config,
            build_virtual_dom: Arc::new(move || VirtualDom::new(root)),
            ssr_state: Default::default(),
        }
    }

    /// Create a new [`RenderHandleState`] with a custom [`VirtualDom`] factory. This method can be used to pass context into the root component of your application.
    pub fn new_with_virtual_dom_factory(
        config: ServeConfig,
        build_virtual_dom: impl Fn() -> VirtualDom + Send + Sync + 'static,
    ) -> Self {
        Self {
            config,
            build_virtual_dom: Arc::new(build_virtual_dom),
            ssr_state: Default::default(),
        }
    }

    /// Set the [`ServeConfig`] for this [`RenderHandleState`]
    pub fn with_config(mut self, config: ServeConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the [`SSRState`] for this [`RenderHandleState`]. Sharing a [`SSRState`] between multiple [`RenderHandleState`]s is more efficient than creating a new [`SSRState`] for each [`RenderHandleState`].
    pub fn with_ssr_state(mut self, ssr_state: SSRState) -> Self {
        self.ssr_state = std::sync::OnceLock::new();
        if self.ssr_state.set(ssr_state).is_err() {
            panic!("SSRState already set");
        }
        self
    }

    fn ssr_state(&self) -> &SSRState {
        self.ssr_state.get_or_init(|| SSRState::new(&self.config))
    }
}

/// SSR renderer handler for Axum with added context injection.
///
/// # Example
/// ```rust,no_run
/// #![allow(non_snake_case)]
/// use std::sync::{Arc, Mutex};
///
/// use axum::routing::get;
/// use dioxus::prelude::*;
///
/// fn app() -> Element {
///     rsx! {
///         "hello!"
///     }
/// }
///
/// #[tokio::main]
/// async fn main() {
///     let addr = dioxus::cli_config::fullstack_address_or_localhost();
///     let router = axum::Router::new()
///         // Register server functions, etc.
///         // Note you can use `register_server_functions_with_context`
///         // to inject the context into server functions running outside
///         // of an SSR render context.
///         .fallback(get(render_handler)
///             .with_state(RenderHandleState::new(ServeConfig::new().unwrap(), app))
///         )
///         .into_make_service();
///     let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
///     axum::serve(listener, router).await.unwrap();
/// }
/// ```
pub async fn render_handler(
    State(state): State<RenderHandleState>,
    request: Request<Body>,
) -> impl IntoResponse {
    let cfg = &state.config;
    let ssr_state = state.ssr_state();
    let build_virtual_dom = {
        let build_virtual_dom = state.build_virtual_dom.clone();
        let context_providers = state.config.context_providers.clone();
        move || {
            let mut vdom = build_virtual_dom();
            for state in context_providers.as_slice() {
                vdom.insert_any_root_context(state());
            }
            vdom
        }
    };

    let (parts, _) = request.into_parts();
    let url = parts
        .uri
        .path_and_query()
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_string();
    let parts: Arc<parking_lot::RwLock<http::request::Parts>> =
        Arc::new(parking_lot::RwLock::new(parts));
    // Create the server context with info from the request
    let server_context = DioxusServerContext::from_shared_parts(parts.clone());
    // Provide additional context from the render state
    add_server_context(&server_context, &state.config.context_providers);

    match ssr_state
        .render(url, cfg, build_virtual_dom, &server_context)
        .await
    {
        Ok((freshness, rx)) => {
            let mut response = axum::response::Html::from(Body::from_stream(rx)).into_response();
            freshness.write(response.headers_mut());
            server_context.send_response(&mut response);
            Result::<http::Response<axum::body::Body>, StatusCode>::Ok(response)
        }
        Err(SSRError::Incremental(e)) => {
            tracing::error!("Failed to render page: {}", e);
            Ok(report_err(e).into_response())
        }
        Err(SSRError::Routing(e)) => {
            tracing::trace!("Page not found: {}", e);
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Page not found"))
                .unwrap())
        }
    }
}

fn report_err<E: std::fmt::Display>(e: E) -> Response<axum::body::Body> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(body::Body::new(format!("Error: {}", e)))
        .unwrap()
}
