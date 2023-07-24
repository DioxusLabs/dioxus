#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_router::prelude::*;

fn main() {
    #[cfg(target_arch = "wasm32")]
    dioxus_web::launch(App);
    #[cfg(not(target_arch = "wasm32"))]
    dioxus_desktop::launch(App);
}

// ANCHOR: router
#[derive(Routable, Clone)]
#[rustfmt::skip]
enum Route {
    #[layout(NavBar)]
        #[route("/")]
        Home {},
        #[nest("/blog")]
            #[layout(Blog)]
            #[route("/")]
            BlogList {},
            #[route("/blog/:name")]
            BlogPost { name: String },
            #[end_layout]
        #[end_nest]
    #[end_layout]
    #[route("/:..route")]
    PageNotFound {
        route: Vec<String>,
    },
}
// ANCHOR_END: router

fn App(cx: Scope) -> Element {
    render! {
        Router {}
    }
}

#[inline_props]
fn NavBar(cx: Scope) -> Element {
    render! {
        nav {
            ul {
                li { Link { to: Route::Home {}, "Home" } }
                li { Link { to: Route::BlogList {}, "Blog" } }
            }
        }
        Outlet {}
    }
}

#[inline_props]
fn Home(cx: Scope) -> Element {
    render! {
        h1 { "Welcome to the Dioxus Blog!" }
    }
}

// ANCHOR: blog
#[inline_props]
fn Blog(cx: Scope) -> Element {
    render! {
        h1 { "Blog" }
        Outlet {}
    }
}
// ANCHOR_END: blog

// ANCHOR: blog_list
#[inline_props]
fn BlogList(cx: Scope) -> Element {
    render! {
        h2 { "Choose a post" }
        ul {
            li {
                Link {
                    to: Route::BlogPost { name: "Blog post 1".into() },
                    "Read the first blog post"
                }
            }
            li {
                Link {
                    to: Route::BlogPost { name: "Blog post 2".into() },
                    "Read the second blog post"
                }
            }
        }
    }
}
// ANCHOR_END: blog_list

// ANCHOR: blog_post
// The name prop comes from the /:name route segment
#[inline_props]
fn BlogPost(cx: Scope, name: String) -> Element {
    render! {
        h2 { "Blog Post: {name}"}
    }
}
// ANCHOR_END: blog_post

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
