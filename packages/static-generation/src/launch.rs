//! This module contains the `launch` function, which is the main entry point for dioxus fullstack

use std::any::Any;

use dioxus_lib::prelude::{Element, VirtualDom};

pub use crate::Config;

/// Launch a fullstack app with the given root component, contexts, and config.
#[allow(unused)]
pub fn launch(
    root: fn() -> Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any + Send + Sync> + Send + Sync>>,
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
            use axum::extract::Path;
            use axum::response::IntoResponse;
            use axum::routing::get;
            use axum::Router;
            use axum::ServiceExt;
            use http::StatusCode;
            use tower_http::services::ServeDir;
            use tower_http::services::ServeFile;

            let github_pages = platform_config.github_pages;
            let path = platform_config.output_dir.clone();
            crate::ssg::generate_static_site(root, platform_config)
                .await
                .unwrap();

            // Serve the program if we are running with cargo
            if std::env::var_os("CARGO").is_some() || std::env::var_os("DIOXUS_ACTIVE").is_some() {
                // Get the address the server should run on. If the CLI is running, the CLI proxies static generation into the main address
                // and we use the generated address the CLI gives us
                let cli_args = dioxus_cli_config::RuntimeCLIArguments::from_cli();
                let address = cli_args
                    .as_ref()
                    .map(|args| args.fullstack_address().address())
                    .unwrap_or_else(|| std::net::SocketAddr::from(([127, 0, 0, 1], 8080)));

                // Point the user to the CLI address if the CLI is running or the fullstack address if not
                let serve_address = cli_args
                    .map(|args| args.cli_address())
                    .unwrap_or_else(|| address);
                println!(
                    "Serving static files from {} at http://{serve_address}",
                    path.display()
                );

                let mut serve_dir =
                    ServeDir::new(path.clone()).call_fallback_on_method_not_allowed(true);

                let mut router = axum::Router::new();

                // If we are acting like github pages, we need to serve the 404 page if the user requests a directory that doesn't exist
                router = if github_pages {
                    router.fallback_service(
                        serve_dir.fallback(ServeFile::new(path.join("404/index.html"))),
                    )
                } else {
                    router.fallback_service(serve_dir.fallback(get(|| async move {
                        "The requested path does not exist"
                            .to_string()
                            .into_response()
                    })))
                };

                let listener = tokio::net::TcpListener::bind(address).await.unwrap();
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
