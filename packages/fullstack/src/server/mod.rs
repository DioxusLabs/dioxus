//! Dioxus utilities for the [Axum](https://docs.rs/axum/latest/axum/index.html) server framework.
//!
//! # Example
//! ```rust, no_run
//! #![allow(non_snake_case)]
//! use dioxus::prelude::*;
//!
//! fn main() {
//!     #[cfg(feature = "web")]
//!     // Hydrate the application on the client
//!     dioxus::launch(app);
//!     #[cfg(feature = "server")]
//!     {
//!         tokio::runtime::Runtime::new()
//!             .unwrap()
//!             .block_on(async move {
//!                 // Get the address the server should run on. If the CLI is running, the CLI proxies fullstack into the main address
//!                 // and we use the generated address the CLI gives us
//!                 let address = dioxus::cli_config::fullstack_address_or_localhost();
//!                 let listener = tokio::net::TcpListener::bind(address)
//!                     .await
//!                     .unwrap();
//!                 axum::serve(
//!                         listener,
//!                         axum::Router::new()
//!                             // Server side render the application, serve static assets, and register server functions
//!                             .serve_dioxus_application(ServeConfigBuilder::default(), app)
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
//!     }
//! }
//!
//! #[server(GetServerData)]
//! async fn get_server_data() -> Result<String, ServerFnError> {
//!     Ok("Hello from the server!".to_string())
//! }
//! ```

pub mod launch;

use std::{collections::HashSet, path::Path};

use axum::{body::Body, extract::Request, response::Response, routing::*};
use dioxus_cli_config::server_config;
use dioxus_lib::prelude::Element;
use http::{header::CACHE_CONTROL, HeaderValue};
use tower::{util::MapResponse, ServiceExt};
use tower_http::services::fs::ServeFileSystemResponseBody;

use crate::prelude::*;

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
    fn serve_dioxus_application<Cfg, Error>(self, cfg: Cfg, app: fn() -> Element) -> Self
    where
        Cfg: TryInto<ServeConfig, Error = Error>,
        Error: std::error::Error,
        Self: Sized;
}

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
        let serve_config = server_config();
        let immutable_assets: HashSet<_> = serve_config
            .immutable_assets()
            .iter()
            .map(|s| s.trim_start_matches("/"))
            .collect();
        let assets_dir = crate::assets_path();

        serve_dir_cached(
            self,
            &public_path,
            &assets_dir,
            &immutable_assets,
            &public_path,
        )
    }

    fn serve_dioxus_application<Cfg, Error>(self, cfg: Cfg, app: fn() -> Element) -> Self
    where
        Cfg: TryInto<ServeConfig, Error = Error>,
        Error: std::error::Error,
    {
        let cfg = cfg.try_into();
        let context_providers = cfg
            .as_ref()
            .map(|cfg| cfg.context_providers.clone())
            .unwrap_or_default();

        // Add server functions and render index.html
        let server = self
            .serve_static_assets()
            .register_server_functions_with_context(context_providers);

        match cfg {
            Ok(cfg) => {
                let ssr_state = SSRState::new(&cfg);
                server.fallback(
                    get(render_handler)
                        .with_state(RenderHandleState::new(cfg, app).with_ssr_state(ssr_state)),
                )
            }
            Err(err) => {
                tracing::trace!("Failed to create render handler. This is expected if you are only using fullstack for desktop/mobile server functions: {}", err);
                server
            }
        }
    }
}

fn serve_dir_cached<S>(
    mut router: Router<S>,
    public_path: &std::path::Path,
    assets_dir: &std::path::Path,
    immutable_assets: &HashSet<&str>,
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
            router = serve_dir_cached(router, public_path, assets_dir, immutable_assets, &path);
        } else {
            let serve_file = ServeFile::new(&path).precompressed_br();
            // All cached assets are served at the root of the asset directory. If we know an asset
            // is hashed for cache busting, we can cache the response on the client side forever. If
            // the asset changes, the hash in the path will also change and the client will refetch it.
            let file_relative_to_asset_dir = path
                .strip_prefix(assets_dir)
                .map(path_components_to_route_lossy)
                .unwrap_or_default();
            let file_relative_to_asset_dir = file_relative_to_asset_dir.trim_start_matches("/");
            if immutable_assets.contains(file_relative_to_asset_dir) {
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

fn cache_response_forever<
    S: ServiceExt<Request<Body>, Response = Response<ServeFileSystemResponseBody>>,
>(
    service: S,
) -> MappedAxumService<S> {
    service.map_response(|mut response: Response<ServeFileSystemResponseBody>| {
        response.headers_mut().insert(
            CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        );
        response
    })
}
