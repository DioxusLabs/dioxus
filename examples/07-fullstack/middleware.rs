//! This example shows how to use middleware in a fullstack Dioxus app.
//!
//! Dioxus supports two ways of middleware:
//! - Applying layers to the top-level axum router
//! - Apply `#[middleware]` attributes to individual handlers

use dioxus::prelude::*;

#[cfg(feature = "server")]
use {std::time::Duration, tower_http::timeout::TimeoutLayer};

fn main() {
    #[cfg(not(feature = "server"))]
    dioxus::launch(app);

    #[cfg(feature = "server")]
    dioxus::serve(|| async move {
        use axum::{extract::Request, middleware::Next};
        use dioxus::server::axum;

        Ok(dioxus::server::router(app)
            // we can apply a layer to the entire router using axum's `.layer` method
            .layer(axum::middleware::from_fn(
                |request: Request, next: Next| async move {
                    println!("Request: {} {}", request.method(), request.uri().path());
                    let res = next.run(request).await;
                    println!("Response: {}", res.status());
                    res
                },
            )))
    });
}

fn app() -> Element {
    let mut per_route = use_action(per_route_middleware);

    rsx! {
        h1 { "Fullstack Middleware Example" }
        button { onclick: move |_| per_route.call(), "Fetch Data" }
        pre { "{per_route.value():#?}" }
    }
}

// We can use the `#[middleware]` attribute to apply middleware to individual handlers.
//
// Here, we're applying a timeout to the `per_route_middleware` handler, which will return a 504
// if the handler takes longer than 3 seconds to complete.
//
// To add multiple middleware layers, simply stack multiple `#[middleware]` attributes.
#[get("/api/count")]
#[middleware(TimeoutLayer::with_status_code(408.try_into().unwrap(), Duration::from_secs(3)))]
async fn per_route_middleware() -> Result<String> {
    tokio::time::sleep(Duration::from_secs(5)).await;
    Ok("Hello, world!".to_string())
}
