#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_router::{Link, Route, Router};
use serde::Deserialize;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {
            ul {
                Link { to: "/",  li { "Go home!" } }
                Link { to: "/users",  li { "List all users" } }
                Link { to: "/blog", li { "Blog posts" } }

                Link { to: "/users/bill",  li { "List all users" } }
                Link { to: "/blog/5", li { "Blog post 5" } }
            }
            Route { to: "/", "Home" }
            Route { to: "/users", "User list" }
            Route { to: "/users/:name", User {} }
            Route { to: "/blog", "Blog list" }
            Route { to: "/blog/:post", BlogPost {} }
            Route { to: "", "Err 404 Route Not Found" }
        }
    })
}

fn BlogPost(cx: Scope) -> Element {
    let post = dioxus_router::use_route(&cx).last_segment()?;

    cx.render(rsx! {
        div {
            h1 { "Reading blog post: {post}" }
            p { "example blog post" }
        }
    })
}

#[derive(Deserialize)]
struct Query {
    bold: bool,
}

fn User(cx: Scope) -> Element {
    let post = dioxus_router::use_route(&cx).last_segment()?;

    let query = dioxus_router::use_route(&cx)
        .query::<Query>()
        .unwrap_or(Query { bold: false });

    cx.render(rsx! {
        div {
            h1 { "Reading blog post: {post}" }
            p { "example blog post" }

            if query.bold {
                rsx!{ b { "bold" } }
            } else {
                rsx!{ i { "italic" } }
            }
        }
    })
}
