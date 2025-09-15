use crate::{
    render::{SSRError, SsrRendererPool},
    with_server_context, DioxusServerContext, ServeConfig,
};
use crate::{ContextProviders, ProvideServerContext};
use axum::body;
use axum::extract::State;
use axum::routing::*;
use axum::{
    body::Body,
    http::{Request, Response, StatusCode},
    response::IntoResponse,
};
use dioxus_isrg::RenderFreshness;
use futures::Stream;

use crate::ServerFunction;
use dioxus_core::{Element, VirtualDom};
use http::header::*;
use std::path::Path;
use std::sync::Arc;
use tower::util::MapResponse;
use tower::ServiceExt;
use tower_http::services::{fs::ServeFileSystemResponseBody, ServeDir};

/// A extension trait with utilities for integrating Dioxus with your Axum router.
pub trait DioxusRouterExt<S>: DioxusRouterFnExt<S> {
    /// Serves the static WASM for your Dioxus application (except the generated index.html).
    ///
    /// # Example
    /// ```rust, no_run
    /// # #![allow(non_snake_case)]
    /// # use dioxus::prelude::*;
    /// use dioxus_server::{DioxusRouterExt, DioxusRouterFnExt};
    ///
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
    /// # use dioxus::prelude::*;
    /// use dioxus_server::{DioxusRouterExt, DioxusRouterFnExt, ServeConfig};
    ///
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
    fn serve_static_assets(self) -> Self {
        let public_path = crate::public_path();

        if !public_path.exists() {
            return self;
        }

        // Serve all files in public folder except index.html
        serve_dir_cached(self, &public_path, &public_path)
    }

    fn serve_dioxus_application(self, cfg: ServeConfig, app: fn() -> Element) -> Self {
        // Add server functions and render index.html
        self.serve_static_assets()
            .register_server_functions_with_context(cfg.context_providers.clone())
            .fallback(
                get(RenderHandleState::render_handler).with_state(RenderHandleState::new(cfg, app)),
            )
    }
}

/// A extension trait with server function utilities for integrating Dioxus with your Axum router.
pub trait DioxusRouterFnExt<S> {
    /// Registers server functions with the default handler. This handler function will pass an empty [`DioxusServerContext`] to your server functions.
    ///
    /// # Example
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # use dioxus_server::DioxusRouterFnExt;
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
    /// # use dioxus::prelude::*;
    /// # use dioxus_server::DioxusRouterFnExt;
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
    /// # use dioxus::prelude::*;
    /// # use dioxus_server::{DioxusRouterFnExt, ServeConfig};
    /// #[tokio::main]
    /// async fn main() {
    ///     let router = axum::Router::new()
    ///         .serve_api_application(ServeConfig::new().unwrap(), app)
    ///         .into_make_service();
    ///     // ...
    /// }
    ///
    /// fn app() -> Element {
    ///     rsx! { "Hello World" }
    /// }
    /// ```
    fn serve_api_application(self, cfg: ServeConfig, app: fn() -> Element) -> Self
    where
        Self: Sized;
}

impl<S: Send + Sync + Clone + 'static> DioxusRouterFnExt<S> for Router<S> {
    fn register_server_functions_with_context(
        mut self,
        context_providers: ContextProviders,
    ) -> Self {
        for f in ServerFunction::collect() {
            self = f.register_server_fn_on_router(self, context_providers.clone());
        }
        self
    }

    fn serve_api_application(self, cfg: ServeConfig, app: fn() -> Element) -> Self
    where
        Self: Sized,
    {
        self.register_server_functions_with_context(cfg.context_providers.clone())
            .fallback(
                get(RenderHandleState::render_handler).with_state(RenderHandleState::new(cfg, app)),
            )
    }
}

/// State used by [`render_handler`] to render a dioxus component with axum
#[derive(Clone)]
pub struct RenderHandleState {
    config: ServeConfig,
    build_virtual_dom: Arc<dyn Fn() -> VirtualDom + Send + Sync>,
    renderers: Arc<SsrRendererPool>,
}

impl RenderHandleState {
    /// Create a new [`RenderHandleState`]
    pub fn new(config: ServeConfig, root: fn() -> Element) -> Self {
        Self {
            renderers: Arc::new(SsrRendererPool::new(4, config.incremental.clone())),
            build_virtual_dom: Arc::new(move || VirtualDom::new(root)),
            config,
        }
    }

    /// Create a new [`RenderHandleState`] with a custom [`VirtualDom`] factory. This method can be used to pass context into the root component of your application.
    pub fn new_with_virtual_dom_factory(
        config: ServeConfig,
        build_virtual_dom: impl Fn() -> VirtualDom + Send + Sync + 'static,
    ) -> Self {
        Self {
            renderers: Arc::new(SsrRendererPool::new(4, config.incremental.clone())),
            config,
            build_virtual_dom: Arc::new(build_virtual_dom),
        }
    }

    /// Set the [`ServeConfig`] for this [`RenderHandleState`]
    pub fn with_config(mut self, config: ServeConfig) -> Self {
        self.config = config;
        self
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
    /// use dioxus_server::{RenderHandleState, render_handler, ServeConfig};
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
        State(state): State<Self>,
        request: Request<Body>,
    ) -> impl IntoResponse {
        let cfg = &state.config;
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
        server_context.add_server_context(&state.config.context_providers);

        match state
            .render(url, cfg, build_virtual_dom, &server_context)
            .await
        {
            Ok((freshness, rx)) => {
                let mut response =
                    axum::response::Html::from(Body::from_stream(rx)).into_response();
                freshness.write(response.headers_mut());
                server_context.send_response(&mut response);
                Result::<http::Response<axum::body::Body>, StatusCode>::Ok(response)
            }
            Err(SSRError::Incremental(e)) => {
                tracing::error!("Failed to render page: {}", e);
                Ok(Self::err_to_axum_response(e).into_response())
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

    fn err_to_axum_response<E: std::fmt::Display>(e: E) -> Response<axum::body::Body> {
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(body::Body::new(format!("Error: {}", e)))
            .unwrap()
    }

    /// Render the application to HTML.
    pub async fn render<'a>(
        &'a self,
        route: String,
        cfg: &'a ServeConfig,
        virtual_dom_factory: impl FnOnce() -> VirtualDom + Send + Sync + 'static,
        server_context: &'a DioxusServerContext,
    ) -> Result<
        (
            RenderFreshness,
            impl Stream<Item = Result<String, dioxus_isrg::IncrementalRendererError>>,
        ),
        SSRError,
    > {
        self.renderers
            .clone()
            .render_to(cfg, route, virtual_dom_factory, server_context)
            .await
    }
}

fn serve_dir_cached<S>(
    mut router: Router<S>,
    public_path: &std::path::Path,
    directory: &std::path::Path,
) -> Router<S>
where
    S: Send + Sync + Clone + 'static,
{
    use tower_http::services::ServeFile;

    let dir = std::fs::read_dir(directory)
        .unwrap_or_else(|e| panic!("Couldn't read public directory at {:?}: {}", &directory, e));

    for entry in dir.flatten() {
        let path = entry.path();
        // Don't serve the index.html file. The SSR handler will generate it.
        if path == public_path.join("index.html") {
            continue;
        }
        let route = path.strip_prefix(public_path).unwrap();
        let route = path_components_to_route_lossy(route);

        if path.is_dir() {
            // router = serve_dir_cached(router, public_path, &path);
            router = router.nest_service(
                &route,
                ServeDir::new(&path)
                    .precompressed_br()
                    .append_index_html_on_directories(false),
            );
        } else {
            let serve_file = ServeFile::new(&path).precompressed_br();
            // All cached assets are served at the root of the asset directory. If we know an asset
            // is hashed for cache busting, we can cache the response on the client side forever. If
            // the asset changes, the hash in the path will also change and the client will refetch it.
            if file_name_looks_immutable(&route) {
                router = router.nest_service(&route, cache_response_forever(serve_file))
            } else {
                router = router.nest_service(&route, serve_file)
            }
        }
    }

    router
}

fn path_components_to_route_lossy(path: &Path) -> String {
    let route = path
        .iter()
        .map(|segment| segment.to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    format!("/{}", route)
}

type MappedAxumService<S> = MapResponse<
    S,
    fn(Response<ServeFileSystemResponseBody>) -> Response<ServeFileSystemResponseBody>,
>;

fn cache_response_forever<S>(service: S) -> MappedAxumService<S>
where
    S: ServiceExt<Request<Body>, Response = Response<ServeFileSystemResponseBody>>,
{
    service.map_response(|mut response: Response<ServeFileSystemResponseBody>| {
        response.headers_mut().insert(
            CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        );
        response
    })
}

fn file_name_looks_immutable(file_name: &str) -> bool {
    // Check if the file name looks like a hash (e.g., "main-dxh12345678.js")
    file_name.rsplit_once("-dxh").is_some_and(|(_, hash)| {
        hash.chars()
            .take_while(|c| *c != '.')
            .all(|c| c.is_ascii_hexdigit())
    })
}

#[test]
fn test_file_name_looks_immutable() {
    assert!(file_name_looks_immutable("main-dxh12345678.js"));
    assert!(file_name_looks_immutable("style-dxhabcdef.css"));
    assert!(!file_name_looks_immutable("index.html"));
    assert!(!file_name_looks_immutable("script.js"));
    assert!(!file_name_looks_immutable("main-dxh1234wyz.js"));
    assert!(!file_name_looks_immutable("main-dxh12345678-invalid.js"));
}
