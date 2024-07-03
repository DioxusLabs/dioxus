//! This module contains the `launch` function, which is the main entry point for dioxus fullstack

use std::{any::Any, sync::Arc};

use dioxus_lib::prelude::{Element, VirtualDom};

pub use crate::Config;

#[allow(unused)]
pub(crate) type ContextProviders = Arc<
    Vec<Box<dyn Fn() -> Box<dyn std::any::Any + Send + Sync + 'static> + Send + Sync + 'static>>,
>;

#[allow(unused)]
fn virtual_dom_factory(
    root: fn() -> Element,
    contexts: ContextProviders,
) -> impl Fn() -> VirtualDom + 'static {
    move || {
        let mut vdom = VirtualDom::new(root);
        for context in &*contexts {
            vdom.insert_any_root_context(context());
        }
        vdom
    }
}

#[cfg(feature = "server")]
/// Launch a fullstack app with the given root component, contexts, and config.
#[allow(unused)]
pub fn launch(
    root: fn() -> Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any + Send + Sync> + Send + Sync>>,
    platform_config: Config,
) -> ! {
    let contexts = Arc::new(contexts);
    let factory = virtual_dom_factory(root, contexts.clone());
    #[cfg(all(feature = "server", not(target_arch = "wasm32")))]
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async move {
            launch_server(platform_config, factory, contexts).await;
        });

    unreachable!("Launching a fullstack app should never return")
}

#[cfg(all(not(feature = "server"), feature = "web"))]
/// Launch a fullstack app with the given root component, contexts, and config.
#[allow(unused)]
pub fn launch(
    root: fn() -> Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any + Send + Sync> + Send + Sync>>,
    platform_config: Config,
) {
    let contexts = Arc::new(contexts);
    let factory = virtual_dom_factory(root, contexts);
    let cfg = platform_config.web_cfg.hydrate(true);
    dioxus_web::launch::launch_virtual_dom(factory(), cfg)
}

#[cfg(all(not(any(feature = "server", feature = "web")), feature = "desktop"))]
/// Launch a fullstack app with the given root component, contexts, and config.
#[allow(unused)]
pub fn launch(
    root: fn() -> Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any + Send + Sync> + Send + Sync>>,
    platform_config: Config,
) -> ! {
    let contexts = Arc::new(contexts);
    let factory = virtual_dom_factory(root, contexts);
    let cfg = platform_config.desktop_cfg;
    dioxus_desktop::launch::launch_virtual_dom(factory(), cfg)
}

#[cfg(all(
    not(any(feature = "server", feature = "web", feature = "desktop")),
    feature = "mobile"
))]
/// Launch a fullstack app with the given root component, contexts, and config.
#[allow(unused)]
pub fn launch(
    root: fn() -> Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any + Send + Sync> + Send + Sync>>,
    platform_config: Config,
) -> ! {
    let contexts = Arc::new(contexts);
    let factory = virtual_dom_factory(root, contexts.clone());
    let cfg = platform_config.mobile_cfg;
    dioxus_mobile::launch::launch_virtual_dom(factory(), cfg)
}

#[cfg(not(any(
    feature = "server",
    feature = "web",
    feature = "desktop",
    feature = "mobile"
)))]
/// Launch a fullstack app with the given root component, contexts, and config.
#[allow(unused)]
pub fn launch(
    root: fn() -> Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any + Send + Sync> + Send + Sync>>,
    platform_config: Config,
) -> ! {
    panic!("No platform feature enabled. Please enable one of the following features: axum, desktop, or web to use the launch API.")
}

#[cfg(feature = "server")]
#[allow(unused)]
/// Launch a server application
async fn launch_server(
    platform_config: Config,
    build_virtual_dom: impl Fn() -> VirtualDom + Send + Sync + 'static,
    context_providers: ContextProviders,
) {
    use clap::Parser;

    use crate::prelude::RenderHandleState;

    let args = dioxus_cli_config::ServeArguments::from_cli()
        .unwrap_or_else(dioxus_cli_config::ServeArguments::parse);
    let addr = args
        .addr
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)));
    let addr = std::net::SocketAddr::new(addr, args.port);
    println!("Listening on http://{}", addr);

    #[cfg(feature = "axum")]
    {
        use crate::axum_adapter::DioxusRouterExt;

        let router = axum::Router::new().register_server_functions_with_context(context_providers);
        #[cfg(not(any(feature = "desktop", feature = "mobile")))]
        let router = {
            use crate::prelude::SSRState;

            let cfg = platform_config.server_cfg.build();

            let mut router = router.serve_static_assets(cfg.assets_path.clone());

            #[cfg(all(feature = "hot-reload", debug_assertions))]
            {
                use dioxus_hot_reload::HotReloadRouterExt;
                router = router.forward_cli_hot_reloading();
            }

            router.fallback(
                axum::routing::get(crate::axum_adapter::render_handler).with_state(
                    RenderHandleState::new_with_virtual_dom_factory(build_virtual_dom)
                        .with_config(cfg),
                ),
            )
        };
        let router = router.into_make_service();
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, router).await.unwrap();
    }
    #[cfg(not(feature = "axum"))]
    {
        panic!("Launching with dioxus fullstack requires the axum feature. If you are using a community fullstack adapter, please check the documentation for that adapter to see how to launch the application.");
    }
}
