#![allow(non_snake_case)]
use dioxus::prelude::*;
use dioxus_router::prelude::*;

// ANCHOR: route
#[derive(Routable, Clone)]
#[rustfmt::skip]
enum Route {
    // Routes always start with a slash
    #[route("/")]
    Home {},
    // You can have multiple segments in a route
    #[route("/hello/world")]
    HelloWorld {},
}

#[inline_props]
fn Home(cx: Scope) -> Element {
    todo!()
}

#[inline_props]
fn HelloWorld(cx: Scope) -> Element {
    todo!()
}
// ANCHOR_END: route

fn main() {}
