#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus::router::{Link, Route, Router};
use serde::Deserialize;

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
                Route { to: ":name", User {} }
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

#[derive(Deserialize)]
struct Query {
    bold: bool,
}

fn User(cx: Scope) -> Element {
    let post = dioxus::router::use_route(&cx).last_segment()?;

    let query = dioxus::router::use_route(&cx)
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
