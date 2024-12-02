//! A launch function that creates an axum router for the LaunchBuilder

use std::any::Any;

use axum::{
    body::Body,
    extract::{Request, State},
    response::IntoResponse,
};
use dioxus_cli_config::base_path;
use dioxus_lib::prelude::*;

use crate::server::{render_handler, RenderHandleState, SSRState};

/// Launch a fullstack app with the given root component, contexts, and config.
#[allow(unused)]
pub fn launch(
    root: fn() -> Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
    platform_config: Vec<Box<dyn Any>>,
) -> ! {
    use crate::{ServeConfig, ServeConfigBuilder};

    #[cfg(not(target_arch = "wasm32"))]
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async move {
            let platform_config = platform_config
                .into_iter()
                .find_map(|cfg| {
                    cfg.downcast::<ServeConfig>()
                        .map(|cfg| Result::Ok(*cfg))
                        .or_else(|cfg| {
                            cfg.downcast::<ServeConfigBuilder>()
                                .map(|builder| builder.build())
                        })
                        .ok()
                })
                .unwrap_or_else(ServeConfig::new);

            // Extend the config's context providers with the context providers from the launch builder
            let platform_config = platform_config.map(|mut cfg| {
                let mut contexts = contexts;
                let cfg_context_providers = cfg.context_providers.clone();
                for i in 0..cfg_context_providers.len() {
                    contexts.push(Box::new({
                        let cfg_context_providers = cfg_context_providers.clone();
                        move || (cfg_context_providers[i])()
                    }));
                }
                cfg.context_providers = std::sync::Arc::new(contexts);
                cfg
            });

            // Get the address the server should run on. If the CLI is running, the CLI proxies fullstack into the main address
            // and we use the generated address the CLI gives us
            let address = dioxus_cli_config::fullstack_address_or_localhost();

            use crate::server::DioxusRouterExt;

            struct TryIntoResult(Result<ServeConfig, crate::UnableToLoadIndex>);

            impl TryInto<ServeConfig> for TryIntoResult {
                type Error = crate::UnableToLoadIndex;

                fn try_into(self) -> Result<ServeConfig, Self::Error> {
                    self.0
                }
            }

            let mut base_path = base_path();
            let config = platform_config.as_ref().ok().cloned();
            let dioxus_router =
                axum::Router::new().serve_dioxus_application(TryIntoResult(platform_config), root);
            let mut router;
            match base_path.as_deref() {
                Some(base_path) => {
                    let base_path = base_path.trim_matches('/');
                    // If there is a base path, nest the router under it and serve the root route manually
                    // Nesting a route in axum only serves /base_path or /base_path/ not both
                    router = axum::Router::new().nest(&format!("/{base_path}/"), dioxus_router);
                    async fn root_render_handler(
                        state: State<RenderHandleState>,
                        mut request: Request<Body>,
                    ) -> impl IntoResponse {
                        // The root of the base path always looks like the root from dioxus fullstack
                        *request.uri_mut() = "/".parse().unwrap();
                        render_handler(state, request).await
                    }
                    if let Some(cfg) = config {
                        let ssr_state = SSRState::new(&cfg);
                        router = router.route(
                            &format!("/{base_path}"),
                            axum::routing::method_routing::get(root_render_handler).with_state(
                                RenderHandleState::new(cfg, root).with_ssr_state(ssr_state),
                            ),
                        )
                    }
                }
                None => router = dioxus_router,
            }

            let router = router.into_make_service();
            let listener = tokio::net::TcpListener::bind(address).await.unwrap();

            axum::serve(listener, router).await.unwrap();
        });

    unreachable!("Launching a fullstack app should never return")
}
