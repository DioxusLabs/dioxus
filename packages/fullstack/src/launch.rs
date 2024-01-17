//! Launch helper macros for fullstack apps
#![allow(unused)]
use crate::prelude::*;
use dioxus_lib::prelude::{
    dioxus_core::{AnyProps, CrossPlatformConfig},
    *,
};

/// The desktop renderer platform
pub struct FullstackPlatform;

impl<Props: AnyProps + Clone + Send + Sync + 'static> dioxus_core::PlatformBuilder<Props>
    for FullstackPlatform
{
    type Config = Config;

    fn launch(config: dioxus_core::CrossPlatformConfig<Props>, platform_config: Self::Config) {
        #[cfg(feature = "ssr")]
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                platform_config.launch_server(config).await;
            });
        #[cfg(not(feature = "ssr"))]
        {
            #[cfg(feature = "web")]
            platform_config.launch_web(config);
            #[cfg(feature = "desktop")]
            platform_config.launch_desktop(config);
        }
    }
}

/// Settings for a fullstack app.
pub struct Config {
    #[cfg(feature = "ssr")]
    server_fn_route: &'static str,
    #[cfg(feature = "ssr")]
    server_cfg: ServeConfigBuilder,
    #[cfg(feature = "ssr")]
    addr: std::net::SocketAddr,
    #[cfg(feature = "web")]
    web_cfg: dioxus_web::Config,
    #[cfg(feature = "desktop")]
    desktop_cfg: dioxus_desktop::Config,
}

#[allow(clippy::derivable_impls)]
impl Default for Config {
    fn default() -> Self {
        Self {
            #[cfg(feature = "ssr")]
            server_fn_route: "",
            #[cfg(feature = "ssr")]
            addr: std::net::SocketAddr::from(([127, 0, 0, 1], 8080)),
            #[cfg(feature = "ssr")]
            server_cfg: ServeConfigBuilder::new(),
            #[cfg(feature = "web")]
            web_cfg: dioxus_web::Config::default(),
            #[cfg(feature = "desktop")]
            desktop_cfg: dioxus_desktop::Config::default(),
        }
    }
}

impl Config {
    /// Create a new config for a fullstack app.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the address to serve the app on.
    #[cfg(feature = "ssr")]
    pub fn addr(self, addr: impl Into<std::net::SocketAddr>) -> Self {
        let addr = addr.into();
        Self { addr, ..self }
    }

    /// Set the route to the server functions.
    #[cfg(feature = "ssr")]
    pub fn server_fn_route(self, server_fn_route: &'static str) -> Self {
        Self {
            server_fn_route,
            ..self
        }
    }

    /// Set the incremental renderer config.
    #[cfg(feature = "ssr")]
    pub fn incremental(self, cfg: IncrementalRendererConfig) -> Self {
        Self {
            server_cfg: self.server_cfg.incremental(cfg),
            ..self
        }
    }

    /// Set the server config.
    #[cfg(feature = "ssr")]
    pub fn server_cfg(self, server_cfg: ServeConfigBuilder) -> Self {
        Self { server_cfg, ..self }
    }

    /// Set the web config.
    #[cfg(feature = "web")]
    pub fn web_cfg(self, web_cfg: dioxus_web::Config) -> Self {
        Self { web_cfg, ..self }
    }

    /// Set the desktop config.
    #[cfg(feature = "desktop")]
    pub fn desktop_cfg(self, desktop_cfg: dioxus_desktop::Config) -> Self {
        Self {
            desktop_cfg,
            ..self
        }
    }

    /// Launch the app.
    pub fn launch<P: AnyProps + Clone + Send + Sync>(self, dioxus_config: CrossPlatformConfig<P>) {
        #[cfg(feature = "ssr")]
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                self.launch_server(dioxus_config).await;
            });
        #[cfg(not(feature = "ssr"))]
        {
            #[cfg(feature = "web")]
            self.launch_web(dioxus_config);
            #[cfg(feature = "desktop")]
            self.launch_desktop(dioxus_config);
        }
    }

    #[cfg(feature = "web")]
    /// Launch the web application
    pub fn launch_web<P: AnyProps>(self, dioxus_config: CrossPlatformConfig<P>) {
        use dioxus_lib::prelude::dioxus_core::{CrossPlatformConfig, PlatformBuilder};

        #[cfg(not(feature = "ssr"))]
        {
            let cfg = self.web_cfg.hydrate(true);
            dioxus_web::WebPlatform::launch(
                // TODO: this should pull the props from the document
                dioxus_config,
                cfg,
            );
        }
    }

    #[cfg(feature = "desktop")]
    /// Launch the web application
    pub fn launch_desktop<P: AnyProps>(self, dioxus_config: CrossPlatformConfig<P>) {
        let cfg = self.desktop_cfg;
        dioxus_desktop::launch_with_props(self.component, self.props, cfg);
    }

    #[cfg(feature = "ssr")]
    /// Launch a server application
    pub async fn launch_server<P: AnyProps + Send + Sync + Clone>(
        self,
        dioxus_config: CrossPlatformConfig<P>,
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
            #[cfg(not(feature = "desktop"))]
            let router = router
                .serve_static_assets(cfg.assets_path)
                .connect_hot_reload()
                .fallback(get(render_handler).with_state((cfg, dioxus_config, ssr_state)));
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
            #[cfg(not(feature = "desktop"))]
            let router = {
                // Serve the dist folder and the index.html file
                let serve_dir = warp::fs::dir(cfg.assets_path);

                router
                    .or(connect_hot_reload())
                    // Then the index route
                    .or(warp::path::end().and(render_ssr(cfg.clone())))
                    // Then the static assets
                    .or(serve_dir)
                    // Then all other routes
                    .or(render_ssr(cfg))
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
            #[cfg(not(feature = "desktop"))]
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
