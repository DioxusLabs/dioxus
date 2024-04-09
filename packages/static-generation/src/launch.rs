//! This module contains the `launch` function, which is the main entry point for dioxus fullstack

use std::any::Any;

use dioxus_lib::prelude::{Element, VirtualDom};

pub use crate::Config;

/// Launch a fullstack app with the given root component, contexts, and config.
#[allow(unused)]
pub fn launch(
    root: fn() -> Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
    platform_config: Config,
) {
    let virtual_dom_factory = move || {
        let mut vdom = VirtualDom::new(root);
        for context in &contexts {
            vdom.insert_any_root_context(context());
        }
        vdom
    };

    #[cfg(feature = "server")]
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async move {
            use axum::routing::get;
            use axum::Router;
            use dioxus_hot_reload::HotReloadRouterExt;
            use tower_http::services::ServeDir;

            let path = platform_config.output_dir.clone();
            crate::ssg::generate_static_site(root, platform_config)
                .await
                .unwrap();

            // Serve the program if we are running with cargo
            if std::env::var_os("CARGO").is_some() || std::env::var_os("DIOXUS_ACTIVE").is_some() {
                println!(
                    "Serving static files from {} at http://127.0.0.1:8080",
                    path.display()
                );
                let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));

                let router = axum::Router::new()
                    .forward_cli_hot_reloading()
                    .nest_service("/", ServeDir::new(path));

                let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
                axum::serve(listener, router.into_make_service())
                    .await
                    .unwrap();
            }
        });

    #[cfg(not(feature = "server"))]
    {
        #[cfg(feature = "web")]
        {
            let cfg = platform_config.web_cfg.hydrate(true);
            dioxus_web::launch::launch_virtual_dom(virtual_dom_factory(), cfg);
        }
    }
}
