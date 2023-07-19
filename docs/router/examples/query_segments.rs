#![allow(non_snake_case, unused)]
use dioxus::prelude::*;
use dioxus_router::prelude::*;

// ANCHOR: route
#[derive(Routable, Clone)]
#[rustfmt::skip]
enum Route {
    // segments that start with ?: are query segments
    #[route("/blog?:name")]
    BlogPost {
        // You must include query segments in child variants
        name: String,
    },
}

// Components must contain the same query segments as their corresponding variant
#[inline_props]
fn BlogPost(cx: Scope, name: String) -> Element {
    todo!()
}
// ANCHOR_END: route

fn main() {}
