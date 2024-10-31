//! A launch function that creates an axum router for the LaunchBuilder

use dioxus_lib::prelude::*;
use std::any::Any;

/// Launch a fullstack app with the given root component, contexts, and config.
#[allow(unused)]
pub fn launch(
    root: fn() -> Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
    platform_config: Vec<Box<dyn Any>>,
) -> ! {
    #[cfg(not(target_arch = "wasm32"))]
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async move {
            use crate::DioxusRouterExt;

            let cfg = platform_config
                .into_iter()
                .find_map(|cfg| cfg.downcast::<crate::ServeConfig>().ok().map(|f| *f))
                .unwrap_or_default();

            // Get the address the server should run on. If the CLI is running, the CLI proxies fullstack into the main address
            // and we use the generated address the CLI gives us
            let address = dioxus_cli_config::fullstack_address_or_localhost();
            let listener = tokio::net::TcpListener::bind(address);
            let app = axum::Router::new()
                .serve_dioxus_application(cfg, root)
                .into_make_service();

            axum::serve(listener.await.unwrap(), app)
                .await
                .expect("App failed while serving")
        });

    unreachable!("Launching a fullstack app should never return")
}
