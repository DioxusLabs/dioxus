//! Run with:
//!
//! ```sh
//! dx build --features web --release
//! cargo run --features server
//! ```

#![allow(unused)]
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

// Generate all routes and output them to the docs path
#[cfg(feature = "server")]
#[tokio::main]
async fn main() {
    let mut renderer = IncrementalRenderer::builder().build();

    generate_static_site(app, &mut renderer, &FullstackHTMLTemplate::default())
        .await
        .unwrap();
}

// Hydrate the page
#[cfg(not(feature = "server"))]
fn main() {
    #[cfg(all(feature = "web", not(feature = "server")))]
    LaunchBuilder::web()
        .with_cfg(dioxus::web::Config::default().hydrate(true))
        .launch(app);
}

fn app() -> Element {
    rsx! {
        Router::<Route> {}
    }
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
