//! Launch helper macros for fullstack apps

#[macro_export]
/// Launch a server with a router
macro_rules! launch_router {
    (@router_config) => {
        dioxus_fullstack::router::FullstackRouterConfig::default()
    };

    (@router_config $router_cfg:expr) => {
        $router_cfg
    };

    (@[$address:expr], $route:ty, $(cfg: $router_cfg:expr,)? {$($rule:ident $(: $cfg:expr)?,)*}) => {
        dioxus_fullstack::launch!(
            @[$address],
            dioxus_fullstack::router::RouteWithCfg::<$route>,
            (dioxus_fullstack::launch_router!(@router_config $($router_cfg)?)),
            {
                $($rule $(: $cfg)?,)*
            }
        )
    };
}

#[macro_export]
/// Launch a server
macro_rules! launch {
    (@web_cfg $server_cfg:ident $wcfg:expr) => {
        #[cfg(feature = "web")]
        let web_cfg = $wcfg;
    };

    (@web_cfg $server_cfg:ident) => {
        #[cfg(feature = "web")]
        let web_cfg = dioxus_web::Config::new();
    };

    (@server_cfg $server_cfg:ident $cfg:expr) => {
        #[cfg(feature = "ssr")]
        let $server_cfg = $cfg;
    };

    (@hot_reload $server_cfg:ident) => {
        #[cfg(feature = "ssr")]
        {
            hot_reload_init!(dioxus_hot_reload::Config::new().with_rebuild_callback(|| {
                std::process::Command::new("cargo")
                    .args(&["run", "--features", "ssr"])
                    .spawn()
                    .unwrap()
                    .wait()
                    .unwrap();
                std::process::Command::new("cargo")
                    .args(&["run", "--features", "web"])
                    .spawn()
                    .unwrap()
                    .wait()
                    .unwrap();
                true
            }));
        }
    };

    (@hot_reload $server_cfg:ident $hot_reload_cfg:expr) => {
        #[cfg(feature = "ssr")]
        {
            hot_reload_init!($hot_reload_cfg);
        }
    };

    (@incremental $server_cfg:ident) => {
        #[cfg(feature = "ssr")]
        let $server_cfg = $server_cfg.incremental(dioxus_fullstack::prelude::IncrementalRendererConfig::default());
    };

    (@incremental $server_cfg:ident $cfg:expr) => {
        #[cfg(feature = "ssr")]
        let $server_cfg = $server_cfg.incremental($cfg);
    };

    (@props_type) => {
        Default::default()
    };

    (@props_type $props:expr) => {
        $props
    };

    (@[$address:expr], $comp:path, $(( $props:expr ),)? {$($rule:ident $(: $cfg:expr)?,)*}) => {
        #[cfg(feature = "web")]
        {
            #[allow(unused)]
            let web_cfg = dioxus_web::Config::new();

            $(
                launch!(@$rule server_cfg $($cfg)?);
            )*

            dioxus_web::launch_with_props(
                $comp,
                dioxus_fullstack::prelude::get_root_props_from_document().expect("Failed to get root props from document"),
                web_cfg.hydrate(true),
            );
        }
        #[cfg(feature = "ssr")]
        {
            let server_cfg = ServeConfigBuilder::new($comp, launch!(@props_type $($props)?));

            $(
                launch!(@$rule server_cfg $($cfg)?);
            )*

            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async move {
                    let addr = std::net::SocketAddr::from($address);

                    dioxus_fullstack::launch::launch_server(addr, server_cfg.build()).await;
                });
        }
    };
}

/// Launch a server with the given configeration
/// This will use the routing intigration of the currently enabled intigration feature
#[cfg(feature = "ssr")]
pub async fn launch_server<P: Clone + serde::Serialize + Send + Sync + 'static>(
    addr: std::net::SocketAddr,
    cfg: crate::prelude::ServeConfig<P>,
) {
    #[cfg(all(feature = "axum", not(feature = "warp"), not(feature = "salvo")))]
    {
        use crate::adapters::axum_adapter::DioxusRouterExt;
        axum::Server::bind(&addr)
            .serve(
                axum::Router::new()
                    .serve_dioxus_application("", cfg)
                    .into_make_service(),
            )
            .await
            .unwrap();
    }
    #[cfg(all(feature = "warp", not(feature = "axum"), not(feature = "salvo")))]
    {
        warp::serve(crate::prelude::serve_dioxus_application("", cfg))
            .run(addr)
            .await;
    }
    #[cfg(all(feature = "salvo", not(feature = "axum"), not(feature = "warp")))]
    {
        use crate::adapters::salvo_adapter::DioxusRouterExt;
        let router = salvo::Router::new().serve_dioxus_application("", cfg);
        salvo::Server::new(salvo::listener::TcpListener::bind(addr))
            .serve(router)
            .await;
    }
}
