//! Run with:
//!
//! ```sh
//! dx build --features web --release
//! cargo run --features server
//! ```

#![allow(unused)]
use dioxus::prelude::*;

// Generate all routes and output them to the static path
fn main() {
    #[cfg(feature = "server")]
    {
        tracing_subscriber::fmt::init();
    }

    launch(|| {
        rsx! {
            Router::<Route> {}
        }
    });
}

#[derive(Clone, Routable, Debug, PartialEq)]
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
