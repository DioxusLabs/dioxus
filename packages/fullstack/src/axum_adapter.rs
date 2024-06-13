//! Dioxus utilities for the [Axum](https://docs.rs/axum/latest/axum/index.html) server framework.
//!
//! # Example
//! ```rust
//! #![allow(non_snake_case)]
//! use dioxus_lib::prelude::*;
//! use dioxus_fullstack::prelude::*;
//!
//! fn main() {
//!     #[cfg(feature = "web")]
//!     // Hydrate the application on the client
//!     dioxus_web::launch_cfg(app, dioxus_web::Config::new().hydrate(true));
//!     #[cfg(feature = "server")]
//!     {
//!         tokio::runtime::Runtime::new()
//!             .unwrap()
//!             .block_on(async move {
//!                 let listener = tokio::net::TcpListener::bind("127.0.0.01:8080")
//!                     .await
//!                     .unwrap();
//!                 axum::serve(
//!                         listener,
//!                         axum::Router::new()
//!                             // Server side render the application, serve static assets, and register server functions
//!                             .serve_dioxus_application("", ServerConfig::new(app, ()))
//!                             .into_make_service(),
//!                     )
//!                     .await
//!                     .unwrap();
//!             });
//!      }
//! }
//!
//! fn app() -> Element {
//!     let mut text = use_signal(|| "...".to_string());
//!
//!     rsx! {
//!         button {
//!             onclick: move |_| async move {
//!                 if let Ok(data) = get_server_data().await {
//!                     text.set(data);
//!                 }
//!             },
//!             "Run a server function"
//!         }
//!         "Server said: {text}"
//!     })
//! }
//!
//! #[server(GetServerData)]
//! async fn get_server_data() -> Result<String, ServerFnError> {
//!     Ok("Hello from the server!".to_string())
//! }
//! ```

use axum::routing::*;
use axum::{
    body::{self, Body},
    extract::State,
    http::{Request, Response, StatusCode},
    response::IntoResponse,
};
use dioxus_lib::prelude::VirtualDom;
use futures_util::Future;
use http::header::*;

use std::sync::Arc;

use crate::prelude::*;

pub(crate) type ContextProviders = Arc<
    Vec<Box<dyn Fn() -> Box<dyn std::any::Any + Send + Sync + 'static> + Send + Sync + 'static>>,
>;

/// A extension trait with utilities for integrating Dioxus with your Axum router.
pub trait DioxusRouterExt<S> {
    /// Registers server functions with the default handler. This handler function will pass an empty [`DioxusServerContext`] to your server functions.
    ///
    /// # Example
    /// ```rust
    /// # use dioxus_lib::prelude::*;
    /// # use dioxus_fullstack::prelude::*;
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
    ///     axum::Server::bind(&addr)
    ///         .serve(
    ///             axum::Router::new()
    ///                 // Register server functions routes with the default handler
    ///                 .register_server_fns("")
    ///                 .into_make_service(),
    ///         )
    ///         .await
    ///         .unwrap();
    /// }
    /// ```
    fn register_server_fns(self, context_providers: ContextProviders) -> Self;

    /// Serves the static WASM for your Dioxus application (except the generated index.html).
    ///
    /// # Example
    /// ```rust
    /// # #![allow(non_snake_case)]
    /// # use dioxus_lib::prelude::*;
    /// # use dioxus_fullstack::prelude::*;
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
    ///     axum::Server::bind(&addr)
    ///         .serve(
    ///             axum::Router::new()
    ///                 // Server side render the application, serve static assets, and register server functions
    ///                 .serve_static_assets("dist")
    ///                 // Server render the application
    ///                 // ...
    ///                 .into_make_service(),
    ///         )
    ///         .await
    ///         .unwrap();
    /// }
    ///
    /// fn app() -> Element {
    ///     unimplemented!()
    /// }
    /// ```
    fn serve_static_assets(
        self,
        assets_path: impl Into<std::path::PathBuf>,
    ) -> impl Future<Output = Self> + Send + Sync
    where
        Self: Sized;

    /// Serves the Dioxus application. This will serve a complete server side rendered application.
    /// This will serve static assets, server render the application, register server functions, and integrate with hot reloading.
    ///
    /// # Example
    /// ```rust
    /// # #![allow(non_snake_case)]
    /// # use dioxus_lib::prelude::*;
    /// # use dioxus_fullstack::prelude::*;
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
    ///     axum::Server::bind(&addr)
    ///         .serve(
    ///             axum::Router::new()
    ///                 // Server side render the application, serve static assets, and register server functions
    ///                 .serve_dioxus_application("", ServerConfig::new(app, ()))
    ///                 .into_make_service(),
    ///         )
    ///         .await
    ///         .unwrap();
    /// }
    ///
    /// fn app() -> Element {
    ///     unimplemented!()
    /// }
    /// ```
    fn serve_dioxus_application(
        self,
        cfg: impl Into<ServeConfig>,
        build_virtual_dom: impl Fn() -> VirtualDom + Send + Sync + 'static,
        context_providers: ContextProviders,
    ) -> impl Future<Output = Self> + Send + Sync
    where
        Self: Sized;
}

impl<S> DioxusRouterExt<S> for Router<S>
where
    S: Send + Sync + Clone + 'static,
{
    fn register_server_fns(mut self, context_providers: ContextProviders) -> Self {
        use http::method::Method;

        let context_providers = Arc::new(context_providers);

        for (path, method) in server_fn::axum::server_fn_paths() {
            tracing::trace!("Registering server function: {} {}", method, path);
            let context_providers = context_providers.clone();
            let handler = move |req| {
                handle_server_fns_inner(
                    path,
                    move || {
                        for context_provider in context_providers.iter() {
                            let context = context_provider();
                            _ = server_context().insert_any(context);
                        }
                    },
                    req,
                )
            };
            self = match method {
                Method::GET => self.route(path, get(handler)),
                Method::POST => self.route(path, post(handler)),
                Method::PUT => self.route(path, put(handler)),
                _ => todo!(),
            };
        }

        self
    }

    // TODO: This is a breaking change, but we should probably serve static assets from a different directory than dist where the server executable is located
    // This would prevent issues like https://github.com/DioxusLabs/dioxus/issues/2327
    fn serve_static_assets(
        mut self,
        assets_path: impl Into<std::path::PathBuf>,
    ) -> impl Future<Output = Self> + Send + Sync {
        use tower_http::services::{ServeDir, ServeFile};

        let assets_path = assets_path.into();
        async move {
            // Serve all files in dist folder except index.html
            let dir = std::fs::read_dir(&assets_path).unwrap_or_else(|e| {
                panic!(
                    "Couldn't read assets directory at {:?}: {}",
                    &assets_path, e
                )
            });

            for entry in dir.flatten() {
                let path = entry.path();
                if path.ends_with("index.html") {
                    continue;
                }
                let route = path
                    .strip_prefix(&assets_path)
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
    }

    fn serve_dioxus_application(
        self,
        cfg: impl Into<ServeConfig>,
        build_virtual_dom: impl Fn() -> VirtualDom + Send + Sync + 'static,
        context_providers: ContextProviders,
    ) -> impl Future<Output = Self> + Send + Sync {
        let cfg = cfg.into();
        async move {
            let ssr_state = SSRState::new(&cfg);

            // Add server functions and render index.html
            let mut server = self
                .serve_static_assets(cfg.assets_path.clone())
                .await
                .register_server_fns(context_providers);

            #[cfg(all(feature = "hot-reload", debug_assertions))]
            {
                use dioxus_hot_reload::HotReloadRouterExt;
                server = server.forward_cli_hot_reloading();
            }

            server.fallback(get(render_handler).with_state((
                cfg,
                Arc::new(build_virtual_dom),
                ssr_state,
            )))
        }
    }
}

fn apply_request_parts_to_response<B>(
    headers: hyper::header::HeaderMap,
    response: &mut axum::response::Response<B>,
) {
    let mut_headers = response.headers_mut();
    for (key, value) in headers.iter() {
        mut_headers.insert(key, value.clone());
    }
}

type AxumHandler<F> = (
    F,
    ServeConfig,
    SSRState,
    Arc<dyn Fn() -> VirtualDom + Send + Sync>,
);

/// SSR renderer handler for Axum with added context injection.
///
/// # Example
/// ```rust,no_run
/// #![allow(non_snake_case)]
/// use std::sync::{Arc, Mutex};
///
/// use axum::routing::get;
/// use dioxus_lib::prelude::*;
/// use dioxus_fullstack::{axum_adapter::render_handler_with_context, prelude::*};
///
/// fn app() -> Element {
///     rsx! {
///         "hello!"
///     }
/// }
///
/// #[tokio::main]
/// async fn main() {
///     let cfg = ServerConfig::new(app, ())
///         .assets_path("dist")
///         .build();
///     let ssr_state = SSRState::new(&cfg);
///
///     // This could be any state you want to be accessible from your server
///     // functions using `[DioxusServerContext::get]`.
///     let state = Arc::new(Mutex::new("state".to_string()));
///
///     let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
///     axum::Server::bind(&addr)
///         .serve(
///             axum::Router::new()
///                 // Register server functions, etc.
///                 // Note you probably want to use `register_server_fns_with_handler`
///                 // to inject the context into server functions running outside
///                 // of an SSR render context.
///                 .fallback(get(render_handler_with_context).with_state((
///                     move |ctx| ctx.insert(state.clone()).unwrap(),
///                     cfg,
///                     ssr_state,
///                 )))
///                 .into_make_service(),
///         )
///         .await
///         .unwrap();
/// }
/// ```
pub async fn render_handler_with_context<F: FnMut(&mut DioxusServerContext)>(
    State((mut inject_context, cfg, ssr_state, virtual_dom_factory)): State<AxumHandler<F>>,
    request: Request<Body>,
) -> impl IntoResponse {
    let (parts, _) = request.into_parts();
    let url = parts.uri.path_and_query().unwrap().to_string();
    let parts: Arc<tokio::sync::RwLock<http::request::Parts>> =
        Arc::new(tokio::sync::RwLock::new(parts));
    let mut server_context = DioxusServerContext::new(parts.clone());
    inject_context(&mut server_context);

    match ssr_state
        .render(url, &cfg, move || virtual_dom_factory(), &server_context)
        .await
    {
        Ok(rendered) => {
            let crate::render::RenderResponse { html, freshness } = rendered;
            let mut response = axum::response::Html::from(html).into_response();
            freshness.write(response.headers_mut());
            let headers = server_context.response_parts().unwrap().headers.clone();
            apply_request_parts_to_response(headers, &mut response);
            response
        }
        Err(e) => {
            tracing::error!("Failed to render page: {}", e);
            report_err(e).into_response()
        }
    }
}

type RenderHandlerExtractor = (
    ServeConfig,
    Arc<dyn Fn() -> VirtualDom + Send + Sync>,
    SSRState,
);

/// SSR renderer handler for Axum
pub async fn render_handler(
    State((cfg, virtual_dom_factory, ssr_state)): State<RenderHandlerExtractor>,
    request: Request<Body>,
) -> impl IntoResponse {
    render_handler_with_context(
        State((|_: &mut _| (), cfg, ssr_state, virtual_dom_factory)),
        request,
    )
    .await
}

fn report_err<E: std::fmt::Display>(e: E) -> Response<axum::body::Body> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(body::Body::new(format!("Error: {}", e)))
        .unwrap()
}

/// A handler for Dioxus server functions. This will run the server function and return the result.
async fn handle_server_fns_inner(
    path: &str,
    additional_context: impl Fn() + 'static + Clone + Send,
    req: Request<Body>,
) -> impl IntoResponse {
    use server_fn::middleware::Service;

    let path_string = path.to_string();

    let future = move || async move {
        let (parts, body) = req.into_parts();
        let req = Request::from_parts(parts.clone(), body);

        if let Some(mut service) =
            server_fn::axum::get_server_fn_service(&path_string)
        {
            let server_context = DioxusServerContext::new(Arc::new(tokio::sync::RwLock::new(parts)));
            additional_context();

            // store Accepts and Referrer in case we need them for redirect (below)
            let accepts_html = req
                .headers()
                .get(ACCEPT)
                .and_then(|v| v.to_str().ok())
                .map(|v| v.contains("text/html"))
                .unwrap_or(false);
            let referrer = req.headers().get(REFERER).cloned();

            // actually run the server fn (which may use the server context)
            let mut res = ProvideServerContext::new(service.run(req), server_context.clone()).await;

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
            let mut res_options = server_context.response_parts_mut().unwrap();
            res.headers_mut().extend(res_options.headers.drain());

            Ok(res)
        } else {
            Response::builder().status(StatusCode::BAD_REQUEST).body(
                {
                    #[cfg(target_family = "wasm")]
                    {
                        Body::from(format!(
                            "No server function found for path: {path_string}\nYou may need to explicitly register the server function with `register_explicit`, rebuild your wasm binary to update a server function link or make sure the prefix your server and client use for server functions match.",
                        ))
                    }
                    #[cfg(not(target_family = "wasm"))]
                    {
                        Body::from(format!(
                            "No server function found for path: {path_string}\nYou may need to rebuild your wasm binary to update a server function link or make sure the prefix your server and client use for server functions match.",
                        ))
                    }
                }
            )
        }
        .expect("could not build Response")
    };
    #[cfg(target_arch = "wasm32")]
    {
        use futures_util::future::FutureExt;

        let result = tokio::task::spawn_local(future);
        let result = result.then(|f| async move { f.unwrap() });
        result.await.unwrap_or_else(|e| {
            use server_fn::error::NoCustomError;
            use server_fn::error::ServerFnErrorSerde;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                ServerFnError::<NoCustomError>::ServerError(e.to_string())
                    .ser()
                    .unwrap_or_default(),
            )
                .into_response()
        })
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        future().await
    }
}
