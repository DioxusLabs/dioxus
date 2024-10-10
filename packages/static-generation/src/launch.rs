//! This module contains the `launch` function, which is the main entry point for dioxus fullstack

use std::any::Any;

use dioxus_lib::prelude::Element;

/// Launch a fullstack app with the given root component, contexts, and config.
#[allow(unused)]
pub fn launch(
    root: fn() -> Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
    platform_config: Vec<Box<dyn Any>>,
) {
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
            let platform_config = platform_config
                .into_iter()
                .find_map(|cfg| cfg.downcast::<crate::Config>().map(|cfg| *cfg).ok())
                .unwrap_or_else(crate::Config::new);

            let github_pages = platform_config.github_pages;
            let path = platform_config.output_dir.clone();
            crate::ssg::generate_static_site(root, platform_config)
                .await
                .unwrap();
        });
}
