//! Launch helper macros for fullstack apps
#![allow(unused)]
use crate::prelude::*;
use dioxus_lib::prelude::*;
use std::sync::Arc;

/// Settings for a fullstack app.
pub struct Config {
    #[cfg(feature = "server")]
    pub(crate) server_fn_route: &'static str,

    #[cfg(feature = "server")]
    pub(crate) server_cfg: ServeConfigBuilder,

    #[cfg(feature = "server")]
    pub(crate) addr: std::net::SocketAddr,

    #[cfg(feature = "web")]
    pub(crate) web_cfg: dioxus_web::Config,

    #[cfg(feature = "desktop")]
    pub(crate) desktop_cfg: dioxus_desktop::Config,

    #[cfg(feature = "mobile")]
    pub(crate) mobile_cfg: dioxus_mobile::Config,
}

#[allow(clippy::derivable_impls)]
impl Default for Config {
    fn default() -> Self {
        Self {
            #[cfg(feature = "server")]
            server_fn_route: "",
            #[cfg(feature = "server")]
            addr: std::net::SocketAddr::from(([127, 0, 0, 1], 8080)),
            #[cfg(feature = "server")]
            server_cfg: ServeConfigBuilder::new(),
            #[cfg(feature = "web")]
            web_cfg: dioxus_web::Config::default(),
            #[cfg(feature = "desktop")]
            desktop_cfg: dioxus_desktop::Config::default(),
            #[cfg(feature = "mobile")]
            mobile_cfg: dioxus_mobile::Config::default(),
        }
    }
}

impl Config {
    /// Create a new config for a fullstack app.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the address to serve the app on.
    #[cfg(feature = "server")]
    #[cfg_attr(any(docsrs, feature = "nightly-doc"), doc(cfg(feature = "server")))]
    pub fn addr(self, addr: impl Into<std::net::SocketAddr>) -> Self {
        let addr = addr.into();
        Self { addr, ..self }
    }

    /// Set the route to the server functions.
    #[cfg(feature = "server")]
    #[cfg_attr(any(docsrs, feature = "nightly-doc"), doc(cfg(feature = "server")))]
    pub fn server_fn_route(self, server_fn_route: &'static str) -> Self {
        Self {
            server_fn_route,
            ..self
        }
    }

    /// Set the incremental renderer config.
    #[cfg(feature = "server")]
    #[cfg_attr(any(docsrs, feature = "nightly-doc"), doc(cfg(feature = "server")))]
    pub fn incremental(self, cfg: IncrementalRendererConfig) -> Self {
        Self {
            server_cfg: self.server_cfg.incremental(cfg),
            ..self
        }
    }

    /// Set the server config.
    #[cfg(feature = "server")]
    #[cfg_attr(any(docsrs, feature = "nightly-doc"), doc(cfg(feature = "server")))]
    pub fn server_cfg(self, server_cfg: ServeConfigBuilder) -> Self {
        Self { server_cfg, ..self }
    }

    /// Set the web config.
    #[cfg(feature = "web")]
    #[cfg_attr(any(docsrs, feature = "nightly-doc"), doc(cfg(feature = "web")))]
    pub fn web_cfg(self, web_cfg: dioxus_web::Config) -> Self {
        Self { web_cfg, ..self }
    }

    /// Set the desktop config.
    #[cfg(feature = "desktop")]
    #[cfg_attr(any(docsrs, feature = "nightly-doc"), doc(cfg(feature = "desktop")))]
    pub fn desktop_cfg(self, desktop_cfg: dioxus_desktop::Config) -> Self {
        Self {
            desktop_cfg,
            ..self
        }
    }

    /// Set the mobile config.
    #[cfg(feature = "mobile")]
    #[cfg_attr(any(docsrs, feature = "nightly-doc"), doc(cfg(feature = "mobile")))]
    pub fn mobile_cfg(self, mobile_cfg: dioxus_mobile::Config) -> Self {
        Self { mobile_cfg, ..self }
    }

    #[cfg(feature = "server")]
    #[cfg_attr(any(docsrs, feature = "nightly-doc"), doc(cfg(feature = "server")))]
    /// Launch a server application
    pub async fn launch_server(
        self,
        build_virtual_dom: impl Fn() -> VirtualDom + Send + Sync + 'static,
    ) {
        let addr = self.addr;
        println!("Listening on {}", addr);
        let cfg = self.server_cfg.build();
        let server_fn_route = self.server_fn_route;
        #[cfg(all(feature = "axum", not(feature = "warp"), not(feature = "salvo")))]
        {
            use crate::adapters::axum_adapter::{render_handler, DioxusRouterExt};
            use axum::routing::get;
            use tower::ServiceBuilder;

            let ssr_state = SSRState::new(&cfg);
            let router = axum::Router::new().register_server_fns(server_fn_route);
            #[cfg(not(any(feature = "desktop", feature = "mobile")))]
            let router = router
                .serve_static_assets(cfg.assets_path.clone())
                .connect_hot_reload()
                .fallback(get(render_handler).with_state((
                    cfg,
                    Arc::new(build_virtual_dom),
                    ssr_state,
                )));
            let router = router
                .layer(
                    ServiceBuilder::new()
                        .layer(tower_http::compression::CompressionLayer::new().gzip(true)),
                )
                .into_make_service();
            axum::Server::bind(&addr).serve(router).await.unwrap();
        }
        #[cfg(all(feature = "warp", not(feature = "axum"), not(feature = "salvo")))]
        {
            use warp::Filter;
            // First register the server functions
            let router = register_server_fns(server_fn_route);
            #[cfg(not(any(feature = "desktop", feature = "mobile")))]
            let router = {
                // Serve the dist folder and the index.html file
                let serve_dir = warp::fs::dir(cfg.assets_path);
                let build_virtual_dom = Arc::new(build_virtual_dom);

                router
                    .or(connect_hot_reload())
                    // Then the index route
                    .or(warp::path::end().and(render_ssr(cfg.clone(), {
                        let build_virtual_dom = build_virtual_dom.clone();
                        move || build_virtual_dom()
                    })))
                    // Then the static assets
                    .or(serve_dir)
                    // Then all other routes
                    .or(render_ssr(cfg, move || build_virtual_dom()))
            };
            warp::serve(router.boxed().with(warp::filters::compression::gzip()))
                .run(addr)
                .await;
        }
        #[cfg(all(feature = "salvo", not(feature = "axum"), not(feature = "warp")))]
        {
            use crate::adapters::salvo_adapter::{DioxusRouterExt, SSRHandler};
            use salvo::conn::Listener;
            let router = salvo::Router::new().register_server_fns(server_fn_route);
            #[cfg(not(any(feature = "desktop", feature = "mobile")))]
            let router = router
                .serve_static_assets(cfg.assets_path)
                .connect_hot_reload()
                .push(salvo::Router::with_path("/<**any_path>").get(SSRHandler::new(cfg)));
            let router = router.hoop(
                salvo::compression::Compression::new()
                    .enable_gzip(salvo::prelude::CompressionLevel::Default),
            );
            salvo::Server::new(salvo::conn::tcp::TcpListener::new(addr).bind().await)
                .serve(router)
                .await;
        }
    }
}
