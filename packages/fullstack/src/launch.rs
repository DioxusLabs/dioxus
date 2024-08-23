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
    #[allow(unused_mut)] mut contexts: Vec<
        Box<dyn Fn() -> Box<dyn Any + Send + Sync> + Send + Sync>,
    >,
    platform_config: Config,
) {
    let contexts = Arc::new(contexts);
    let mut factory = virtual_dom_factory(root, contexts);
    let cfg = platform_config.web_cfg.hydrate(true);

    #[cfg(feature = "document")]
    let factory = move || {
        let mut vdom = factory();
        todo!("Fullstack web document???");
        // let document = std::rc::Rc::new(crate::document::web::FullstackWebDocument)
        //     as std::rc::Rc<dyn dioxus_document::Document>;
        // vdom.provide_root_context(document);
        vdom
    };

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
    // Get the address the server should run on. If the CLI is running, the CLI proxies fullstack into the main address
    // and we use the generated address the CLI gives us
    let serve_address = dioxus_runtime_config::fullstack_address()
        .unwrap_or_else(|| todo!("parse cli args for fullstack address"));

    #[cfg(feature = "axum")]
    {
        use crate::axum_adapter::DioxusRouterExt;

        #[allow(unused_mut)]
        let mut router =
            axum::Router::new().register_server_functions_with_context(context_providers);

        #[cfg(not(any(feature = "desktop", feature = "mobile")))]
        {
            use crate::prelude::RenderHandleState;
            use crate::prelude::SSRState;

            match platform_config.server_cfg.build() {
                Ok(cfg) => {
                    router = router.serve_static_assets();

                    router = router.fallback(
                        axum::routing::get(crate::axum_adapter::render_handler).with_state(
                            RenderHandleState::new_with_virtual_dom_factory(cfg, build_virtual_dom),
                        ),
                    );
                }
                Err(err) => {
                    tracing::trace!("Failed to create render handler. This is expected if you are only using fullstack for desktop/mobile server functions: {}", err);
                }
            }
        }

        let router = router.into_make_service();
        let listener = tokio::net::TcpListener::bind(address).await.unwrap();

        axum::serve(listener, router).await.unwrap();
    }
    #[cfg(not(feature = "axum"))]
    {
        panic!("Launching with dioxus fullstack requires the axum feature. If you are using a community fullstack adapter, please check the documentation for that adapter to see how to launch the application.");
    }
}
