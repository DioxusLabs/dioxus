//! You can use the `github_pages` method to set up a preset for github pages.
//! This will output your files in the `/docs` directory and set up a `404.html` file.

#![allow(unused)]
use dioxus::prelude::*;

// Generate all routes and output them to the static path
fn main() {
    LaunchBuilder::new()
        .with_cfg(dioxus::static_site_generation::Config::new().github_pages())
        .launch(|| {
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

    // You must include a catch all route to handle 404s
    #[route("/:..route")]
    PageNotFound { route: Vec<String> },
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

    rsx! {
        Link { to: Route::Blog {}, "Go to blog" }
        div {
            h1 { "High-Five counter: {count}" }
            button { onclick: move |_| count += 1, "Up high!" }
            button { onclick: move |_| count -= 1, "Down low!" }
        }
    }
}

#[component]
fn PageNotFound(route: Vec<String>) -> Element {
    rsx! {
        h1 { "Page not found" }
        p { "We are terribly sorry, but the page you requested doesn't exist." }
        pre { color: "red", "log:\nattempted to navigate to: {route:?}" }
    }
}
