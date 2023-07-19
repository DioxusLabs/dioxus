// ANCHOR: router
#![allow(non_snake_case)]
use dioxus::prelude::*;
use dioxus_router::prelude::*;

/// An enum of all of the possible routes in the app.
#[derive(Routable, Clone)]
enum Route {
    // The home page is at the / route
    #[route("/")]
    // If the name of the component and variant are the same you can omit the component and props name
    // If they are different you can specify them like this:
    // #[route("/", ComponentName, PropsName)]
    Home {},
}
// ANCHOR_END: router

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

fn main() {}
