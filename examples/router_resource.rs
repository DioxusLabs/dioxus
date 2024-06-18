//! Example: Updating components with use_resource
//! -----------------
//!
//! This example shows how to use use_reactive to update a component properly
//! when linking to it from the same component, when using use_resource

use dioxus::prelude::*;

#[derive(Clone, Routable, Debug, PartialEq)]
enum Route {
    #[route("/")]
    Home {},
    #[route("/blog/:id")]
    Blog { id: i32 },
}

fn main() {
    launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        Router::<Route> {}
    }
}

#[component]
fn Blog(id: ReactOnlySignal<i32>) -> Element {
    async fn future(n: i32) -> i32 {
        n
    }

    // if you use the naive approach, the "Blog post {id}" below will never update when clicking links!
    // let res = use_resource(move || future(id));

    // the use_reactive hook is required to properly update when clicking links to this component, from this component
    let res = use_resource(move || future(id()));

    match res() {
        Some(id) => rsx! {
            div {
                "Blog post {id}"
            }
            for i in 0..10 {
                div {
                    Link { to: Route::Blog { id: i }, "Go to Blog {i}" }
                }
            }
        },
        None => rsx! {},
    }
}

#[component]
fn Home() -> Element {
    rsx! {
        Link {
            to: Route::Blog {
                id: 0
            },
            "Go to blog"
        }
    }
}
