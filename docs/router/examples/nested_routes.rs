#![allow(non_snake_case)]
use dioxus::prelude::*;
use dioxus_router::prelude::*;

// ANCHOR: router
#[derive(Routable, Clone)]
#[rustfmt::skip]
enum Route {
    // All routes under the NavBar layout will be rendered inside of the NavBar Outlet
    #[layout(NavBar)]
        #[route("/")]
        Home {},
    #[end_layout]
    #[route("/:...route")]
    PageNotFound { route: Vec<String> },
}
// ANCHOR_END: router

// ANCHOR: nav
#[inline_props]
fn NavBar(cx: Scope) -> Element {
    render! {
        nav {
            ul {
                li { "links" }
            }
        }
        // The Outlet component will render child routes (In this case just the Home component) inside the Outlet component
        Outlet {}
    }
}
// ANCHOR_END: nav

// ANCHOR: app
#[inline_props]
fn App(cx: Scope) -> Element {
    render! {
        Router {}
    }
}
// ANCHOR_END: app

// ANCHOR: home
#[inline_props]
fn Home(cx: Scope) -> Element {
    render! {
        h1 { "Welcome to the Dioxus Blog!" }
    }
}
// ANCHOR_END: home

// ANCHOR: fallback
#[inline_props]
fn PageNotFound(cx: Scope, route: Vec<String>) -> Element {
    render! {
        h1 { "Page not found" }
        p { "We are terribly sorry, but the page you requested doesn't exist." }
        pre {
            color: "red",
            "log:\nattemped to navigate to: {route:?}"
        }
    }
}
// ANCHOR_END: fallback

fn main() {}
