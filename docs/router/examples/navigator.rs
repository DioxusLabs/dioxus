#![allow(non_snake_case)]
use dioxus::prelude::*;
use dioxus_router::prelude::*;

#[derive(Routable, Clone)]
#[rustfmt::skip]
enum Route {
    #[route("/")]
    Home {},
    #[route("/:..route")]
    PageNotFound { route: Vec<String> },
}

#[inline_props]
fn App(cx: Scope) -> Element {
    render! {
        Router {}
    }
}

// ANCHOR: nav
#[inline_props]
fn Home(cx: Scope) -> Element {
    let nav = use_navigator(cx);

    // push
    nav.push(Route::PageNotFound { route: vec![] });

    // replace
    nav.replace(Route::Home {});

    // go back
    nav.go_back();

    // go forward
    nav.go_forward();

    render! {
        h1 { "Welcome to the Dioxus Blog!" }
    }
}
// ANCHOR_END: nav

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

fn main() {}
