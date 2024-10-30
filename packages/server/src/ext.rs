use axum::routing::*;
use axum::{
    body::{self, Body},
    extract::State,
    http::{Request, Response, StatusCode},
    response::IntoResponse,
};
use dioxus_lib::prelude::{Element, VirtualDom};
use http::header::*;
use parking_lot::RwLock;

use std::{sync::Arc, task::Poll};

use crate::{
    handle_server_fns_inner, render_handler, ContextProviders, DioxusServerContext,
    IncrementalRendererError, ProvideServerContext, RenderHandleState, ServeConfig, SsrRenderer,
};

/// A extension trait with utilities for integrating Dioxus with your Axum router.
pub trait DioxusRouterExt<S> {
    /// Registers server functions with the default handler. This handler function will pass an empty [`DioxusServerContext`] to your server functions.
    ///
    /// # Example
    /// ```rust, no_run
    /// # use dioxus_lib::prelude::*;
    /// # use dioxus_fullstack::prelude::*;
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
    ///     let router = axum::Router::new()
    ///         // Register server functions routes with the default handler
    ///         .register_server_functions()
    ///         .into_make_service();
    ///     let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    ///     axum::serve(listener, router).await.unwrap();
    /// }
    /// ```
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
    ///     let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
    ///     let router = axum::Router::new()
    ///         // Register server functions routes with the default handler
    ///         .register_server_functions_with_context(Arc::new(vec![Box::new(|| Box::new(1234567890u32))]))
    ///         .into_make_service();
    ///     let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    ///     axum::serve(listener, router).await.unwrap();
    /// }
    /// ```
    fn register_server_functions_with_context(self, context_providers: ContextProviders) -> Self;

    /// Serves the static WASM for your Dioxus application (except the generated index.html).
    ///
    /// # Example
    /// ```rust, no_run
    /// # #![allow(non_snake_case)]
    /// # use dioxus_lib::prelude::*;
    /// # use dioxus_fullstack::prelude::*;
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
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
    ///     let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
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
    fn register_server_functions_with_context(
        mut self,
        context_providers: ContextProviders,
    ) -> Self {
        use http::method::Method;

        for (path, method) in server_fn::axum::server_fn_paths() {
            tracing::trace!("Registering server function: {} {}", method, path);
            let context_providers = context_providers.clone();
            let handler = move |req| {
                handle_server_fns_inner(
                    path,
                    move |server_context| {
                        for index in 0..context_providers.len() {
                            let context_providers = context_providers.clone();
                            server_context
                                .insert_boxed_factory(Box::new(move || context_providers[index]()));
                        }
                    },
                    req,
                )
            };
            self = match method {
                Method::GET => self.route(path, get(handler)),
                Method::POST => self.route(path, post(handler)),
                Method::PUT => self.route(path, put(handler)),
                _ => unimplemented!("Unsupported server function method: {}", method),
            };
        }

        self
    }

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

    fn serve_dioxus_application<Cfg, Error>(self, cfg: Cfg, app: fn() -> Element) -> Self
    where
        Cfg: TryInto<ServeConfig, Error = Error>,
        Error: std::error::Error,
    {
        // Add server functions and render index.html
        let server = self.serve_static_assets().register_server_functions();

        match cfg.try_into() {
            Ok(cfg) => {
                let ssr_state = SsrRenderer::shared(cfg.incremental.clone());
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
