use dioxus::prelude::*;

#[tokio::main]
async fn main() {
    // We can add a new axum router that gets merged with the Dioxus one.
    // Because this is in a closure, you get hot-patching.
    #[cfg(feature = "server")]
    dioxus_fullstack::with_axum_router(|| async move {
        use axum::routing::{get, post};

        let router = axum::Router::new()
            .route("/", get(|| async { "Hello, world!" }))
            .route("/submit", post(|| async { "Form submitted!" }))
            .route("/about", get(|| async { "About us" }))
            .route("/contact", get(|| async { "Contact us" }));

        anyhow::Ok(router)
    });

    // That router has priority over the Dioxus one, so you can do things like middlewares easily
    dioxus::launch(|| {
        rsx! {
            div { "Hello, world!" }
        }
    });
}
