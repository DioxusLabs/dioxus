//! Example: Updating components with use_resource
//! -----------------
//!
//! This example shows how to use ReadOnlySignal to make props reactive
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

// We use id: ReadOnlySignal<i32> instead of id: i32 to make id work with reactive hooks
// Any i32 we pass in will automatically be converted into a ReadOnlySignal<i32>
#[component]
fn Blog(id: ReadOnlySignal<i32>) -> Element {
    async fn future(n: i32) -> i32 {
        n
    }

    // Because we accept ReadOnlySignal<i32> instead of i32, the resource will automatically subscribe to the id when we read it
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
