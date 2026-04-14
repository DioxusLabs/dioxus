//! This example demonstrates how to use `dioxus::serve` with a custom Axum router.
//!
//! By default, `dioxus::launch` takes over the main thread and runs the Dioxus application.
//! However, if you want to integrate Dioxus into an existing web server or use a custom router,
//! you can use `dioxus::serve` to create a server that serves your Dioxus application alongside
//! other routes.
//!
//! `dioxus::serve` sets up an async runtime, logging, hot-reloading, crash handling, and more.
//! You can then use the `.serve_dioxus_application` method on your router to serve the Dioxus app.
//!
//! `dioxus::serve` is most useful for customizing the server setup, such as adding middleware,
//! custom routes, or integrating with existing axum backend code.
//!
//! Note that `dioxus::serve` is accepts a Router from `axum`. Dioxus will use the IP and PORT
//! environment variables to determine where to bind the server. To customize the port, use environment
//! variables or a `.env` file.
//!
//! On other platforms (like desktop or mobile), you'll want to use `dioxus::launch` instead and then
//! handle async loading of data through hooks like `use_future` or `use_resource` and give the user
//! a loading state while data is being fetched.

use dioxus::prelude::*;

fn main() {
    // On the client we just launch the app as normal.
    #[cfg(not(feature = "server"))]
    dioxus::launch(app);

    // On the server, we can use `dioxus::serve` and `.serve_dioxus_application` to serve our app with routing.
    // The `dioxus::server::router` function creates a new axum Router with the necessary routes to serve the Dioxus app.
    #[cfg(feature = "server")]
    dioxus::serve(|| async move {
        use dioxus::server::axum::routing::{get, post};

        Ok(dioxus::server::router(app)
            .route("/submit", post(|| async { "Form submitted!" }))
            .route("/about", get(|| async { "About us" }))
            .route("/contact", get(|| async { "Contact us" })))
    });
}

fn app() -> Element {
    rsx! {
        div { "Hello from Dioxus!" }
    }
}
