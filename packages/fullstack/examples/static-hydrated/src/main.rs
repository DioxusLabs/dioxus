//! Run with:
//!
//! ```sh
//! dx build --features web --release
//! cargo run --features server
//! ```

#![allow(unused)]
use dioxus::prelude::*;
use dioxus_fullstack::{launch, prelude::*};
use serde::{Deserialize, Serialize};

// Generate all routes and output them to the docs path
#[cfg(feature = "server")]
#[tokio::main]
async fn main() {
    let wrapper = DefaultRenderer {
        before_body: r#"<!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width,
        initial-scale=1.0">
        <title>Dioxus Application</title>
    </head>
    <body>"#
            .to_string(),
        after_body: r#"</body>
    </html>"#
            .to_string(),
    };
    let mut renderer = IncrementalRenderer::builder().build();
    pre_cache_static_routes::<Route, _>(&mut renderer, &wrapper)
        .await
        .unwrap();
}

// Hydrate the page
#[cfg(all(feature = "web", not(feature = "server")))]
fn main() {
    dioxus_web::launch_with_props(
        dioxus_fullstack::router::RouteWithCfg::<Route>,
        dioxus_fullstack::prelude::get_root_props_from_document()
            .expect("Failed to get root props from document"),
        dioxus_web::Config::default().hydrate(true),
    );
}

#[derive(Clone, Routable, Debug, PartialEq, Serialize, Deserialize)]
enum Route {
    #[route("/")]
    Home {},

    #[route("/blog")]
    Blog,
}

#[component]
fn Blog() -> Element {
    rsx! {
        Link { to: Route::Home {}, "Go to counter" }
        table {
            tbody {
                for _ in 0..100 {
                    tr {
                        for _ in 0..100 {
                            td { "hello world!" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Home() -> Element {
    let mut count = use_signal(|| 0);
    let text = use_signal(|| "...".to_string());

    rsx! {
        Link { to: Route::Blog {}, "Go to blog" }
        div {
            h1 { "High-Five counter: {count}" }
            button { onclick: move |_| count += 1, "Up high!" }
            button { onclick: move |_| count -= 1, "Down low!" }
        }
    }
}
