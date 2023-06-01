#![allow(non_snake_case)]
use dioxus::prelude::*;
use dioxus_router::prelude::*;

#[derive(Routable, Clone)]
enum Route {
    #[route("/")]
    Home {},
}

// ANCHOR: app
#[inline_props]
fn App(cx: Scope) -> Element {
    render! {
        Router {
            config: || RouterConfig::default().history(WebHistory::default())
        }
    }
}
// ANCHOR_END: app

#[inline_props]
fn Home(cx: Scope) -> Element {
    render! {
        h1 { "Welcome to the Dioxus Blog!" }
    }
}

fn main() {}
