//! Launch helper macros for fullstack apps
#![allow(unused)]
use crate::prelude::*;
use dioxus::prelude::*;
#[cfg(feature = "router")]
use dioxus_router::prelude::*;

/// A builder for a fullstack app.
pub struct LaunchBuilder<Props: Clone> {
    component: Component<Props>,
    #[cfg(not(feature = "ssr"))]
    props: Props,
    #[cfg(feature = "ssr")]
    server_fn_route: &'static str,
    #[cfg(feature = "ssr")]
    server_cfg: ServeConfigBuilder<Props>,
    #[cfg(feature = "ssr")]
    addr: std::net::SocketAddr,
    #[cfg(feature = "web")]
    web_cfg: dioxus_web::Config,
    #[cfg(feature = "desktop")]
    desktop_cfg: dioxus_desktop::Config,
}

impl<Props: Clone + serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static>
    LaunchBuilder<Props>
{
    /// Create a new builder for a fullstack app.
    pub fn new(component: Component<Props>) -> Self
    where
        Props: Default,
    {
        Self::new_with_props(component, Default::default())
    }

    /// Create a new builder for a fullstack app with props.
    pub fn new_with_props(component: Component<Props>, props: Props) -> Self
    where
        Props: Default,
    {
        Self {
            component,
            #[cfg(not(feature = "ssr"))]
            props,
            #[cfg(feature = "ssr")]
            server_fn_route: "",
            #[cfg(feature = "ssr")]
            addr: std::net::SocketAddr::from(([127, 0, 0, 1], 8080)),
            #[cfg(feature = "ssr")]
            server_cfg: ServeConfigBuilder::new(component, props),
            #[cfg(feature = "web")]
            web_cfg: dioxus_web::Config::default(),
            #[cfg(feature = "desktop")]
            desktop_cfg: dioxus_desktop::Config::default(),
        }
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
    pub fn server_cfg(self, server_cfg: ServeConfigBuilder<Props>) -> Self {
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
    pub fn launch(self) {
        #[cfg(feature = "ssr")]
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                self.launch_server().await;
            });
        #[cfg(not(feature = "ssr"))]
        {
            #[cfg(feature = "web")]
            self.launch_web();
            #[cfg(feature = "desktop")]
            self.launch_desktop();
        }
    }

    #[cfg(feature = "web")]
    /// Launch the web application
    pub fn launch_web(self) {
        #[cfg(not(feature = "ssr"))]
        {
            let cfg = self.web_cfg.hydrate(true);
            dioxus_web::launch_with_props(
                self.component,
                get_root_props_from_document().unwrap(),
                cfg,
            );
        }
    }

    #[cfg(feature = "desktop")]
    /// Launch the web application
    pub fn launch_desktop(self) {
        let cfg = self.desktop_cfg;
        dioxus_desktop::launch_with_props(self.component, self.props, cfg);
    }

    #[cfg(feature = "ssr")]
    /// Launch a server application
    pub async fn launch_server(self) {
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
                .fallback(get(render_handler).with_state((cfg, ssr_state)));
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

#[cfg(feature = "router")]
impl<R: Routable> LaunchBuilder<crate::router::FullstackRouterConfig<R>>
where
    <R as std::str::FromStr>::Err: std::fmt::Display,
    R: Clone + serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static,
{
    /// Create a new launch builder for the given router.
    pub fn router() -> Self {
        let component = crate::router::RouteWithCfg::<R>;
        let props = crate::router::FullstackRouterConfig::default();
        Self::new_with_props(component, props)
    }
}
