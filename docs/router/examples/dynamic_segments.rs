#![allow(unused)]
use dioxus::prelude::*;
use dioxus_router::prelude::*;

// ANCHOR: route
#[derive(Routable, Clone)]
#[rustfmt::skip]
enum Route {
    // segments that start with : are dynamic segments
    #[route("/blog/:name")]
    BlogPost {
        // You must include dynamic segments in child variants
        name: String,
    },
    #[route("/document/:id")]
    Document {
        // You can use any type that implements FromStr
        // If the segment can't be parsed, the route will not match
        id: usize,
    },
}

// Components must contain the same dynamic segments as their corresponding variant
#[component]
fn BlogPost(cx: Scope, name: String) -> Element {
    todo!()
}

#[component]
fn Document(cx: Scope, id: usize) -> Element {
    todo!()
}
// ANCHOR_END: route

fn main() {}
