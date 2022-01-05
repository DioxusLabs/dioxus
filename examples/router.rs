#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus::router::{Link, Route, Router};

fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {
            ul {
                Link { to: "/",  li { "Go home!" } }
                Link { to: "users",  li { "List all users" } }
                Link { to: "blog", li { "Blog posts" } }
            }
            Route { to: "/", "Home" }
            Route { to: "users",
                Route { to: "/", "User list" }
                Route { to: ":name", BlogPost {} }
             }
            Route { to: "blog"
                Route { to: "/", "Blog list" }
                Route { to: ":post", BlogPost {} }
            }
            Route { to: "", "Err 404 Route Not Found" }
        }
    })
}

fn BlogPost(cx: Scope) -> Element {
    let post = dioxus::router::use_route(&cx).last_segment()?;

    cx.render(rsx! {
        div {
            h1 { "Reading blog post: {post}" }
            p { "example blog post" }
        }
    })
}

fn User(cx: Scope) -> Element {
    let post = dioxus::router::use_route(&cx).last_segment()?;
    let bold = dioxus::router::use_route(&cx).param::<bool>("bold");

    cx.render(rsx! {
        div {
            h1 { "Reading blog post: {post}" }
            p { "example blog post" }
        }
    })
}
