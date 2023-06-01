#![allow(non_snake_case, unused)]
use dioxus::prelude::*;
use dioxus_router::prelude::*;

// ANCHOR: route
#[derive(Routable, Clone)]
// Skipping formatting allows you to indent nests
#[rustfmt::skip]
enum Route {
    // Start the /blog nest
    #[nest("/blog")]
        // You can nest as many times as you want
        #[nest("/:id")]
            #[route("/post")]
            PostId {
                // You must include parent dynamic segments in child variants
                id: usize,
            },
        // End nests manually with #[end_nest]
        #[end_nest]
        #[route("/:id")]
        // The absolute route of BlogPost is /blog/:name
        BlogPost {
            id: usize,
        },
    // Or nests are ended automatically at the end of the enum
}

#[inline_props]
fn BlogPost(cx: Scope, id: usize) -> Element {
    todo!()
}

#[inline_props]
fn PostId(cx: Scope, id: usize) -> Element {
    todo!()
}
// ANCHOR_END: route

fn main() {}
