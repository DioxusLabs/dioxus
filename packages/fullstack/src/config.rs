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
    #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
    pub fn addr(self, addr: impl Into<std::net::SocketAddr>) -> Self {
        let addr = addr.into();
        Self { addr, ..self }
    }

    /// Set the route to the server functions.
    #[cfg(feature = "server")]
    #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
    pub fn server_fn_route(self, server_fn_route: &'static str) -> Self {
        Self {
            server_fn_route,
            ..self
        }
    }

    /// Set the incremental renderer config.
    #[cfg(feature = "server")]
    #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
    pub fn incremental(self, cfg: IncrementalRendererConfig) -> Self {
        Self {
            server_cfg: self.server_cfg.incremental(cfg),
            ..self
        }
    }

    /// Set the server config.
    #[cfg(feature = "server")]
    #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
    pub fn server_cfg(self, server_cfg: ServeConfigBuilder) -> Self {
        Self { server_cfg, ..self }
    }

    /// Set the web config.
    #[cfg(feature = "web")]
    #[cfg_attr(docsrs, doc(cfg(feature = "web")))]
    pub fn web_cfg(self, web_cfg: dioxus_web::Config) -> Self {
        Self { web_cfg, ..self }
    }

    /// Set the desktop config.
    #[cfg(feature = "desktop")]
    #[cfg_attr(docsrs, doc(cfg(feature = "desktop")))]
    pub fn desktop_cfg(self, desktop_cfg: dioxus_desktop::Config) -> Self {
        Self {
            desktop_cfg,
            ..self
        }
    }

    /// Set the mobile config.
    #[cfg(feature = "mobile")]
    #[cfg_attr(docsrs, doc(cfg(feature = "mobile")))]
    pub fn mobile_cfg(self, mobile_cfg: dioxus_mobile::Config) -> Self {
        Self { mobile_cfg, ..self }
    }

    #[cfg(feature = "server")]
    #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
    /// Launch a server application
    pub async fn launch_server(
        self,
        build_virtual_dom: impl Fn() -> VirtualDom + Send + Sync + 'static,
    ) {
        let addr = self.addr;
        println!("Listening on {}", addr);
        let cfg = self.server_cfg.build();
        let server_fn_route = self.server_fn_route;

        #[cfg(feature = "axum")]
        {
            use crate::axum_adapter::{render_handler, DioxusRouterExt};
            use axum::routing::get;
            use tower::ServiceBuilder;

            let ssr_state = SSRState::new(&cfg);
            let router = axum::Router::new().register_server_fns();
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
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            axum::serve(listener, router).await.unwrap();
        }
    }
}
