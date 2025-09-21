use dioxus::prelude::*;

fn main() {
    // On the client we just launch the app as normal.
    #[cfg(not(feature = "server"))]
    dioxus::launch(app);

    // On the server, we can use `dioxus::serve` and `.serve_dioxus_application` to serve our app with routing.
    // Using `dioxus::serve` sets up an async runtime, logging, hot-reloading, and more.
    #[cfg(feature = "server")]
    dioxus_server::serve(|| async move {
        use dioxus_server::axum::{
            self,
            routing::{get, post},
        };

        let router = axum::Router::new()
            .serve_dioxus_application(ServeConfig::new().unwrap(), app)
            .route("/", get(|| async { "Hello, world!" }))
            .route("/submit", post(|| async { "Form submitted!" }))
            .route("/about", get(|| async { "About us" }))
            .route("/contact", get(|| async { "Contact us" }));

        anyhow::Ok(router)
    });
}

fn app() -> Element {
    rsx! {
        div { "Hello from Dioxus!" }
    }
}
