#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_router::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::new()
            .index(Home as Component)
            .fixed(
                "blog",
                Route::new(()).nested(
                    Segment::new()
                        .index(BlogList as Component)
                        .catch_all(("post", BlogPost as Component)),
                ),
            )
            .fixed(
                "users",
                Route::new(()).nested(
                    Segment::new()
                        .index(UserList as Component)
                        .catch_all(("name", User as Component)),
                ),
            )
            .fallback(RouteNotFound as Component)
    });

    cx.render(rsx! {
        Router {
            routes: routes.clone(),

            ul {
                Link { target: "/",  li { "Go home!" } }

                Link { target: "/users",  li { "List all users" } }
                Link { target: "/users/bill",  li { "Show user \"bill\"" } }
                Link { target: "/users/franz?bold", li { "Show user \"franz\""}}

                Link { target: "/blog", li { "List all blog posts" } }
                Link { target: "/blog/5", li { "Blog post 5" } }
            }
            Outlet { }
        }
    })
}

fn Home(cx: Scope) -> Element {
    cx.render(rsx! {
        "Home"
    })
}

fn BlogList(cx: Scope) -> Element {
    cx.render(rsx! {
        "Blog list"
    })
}

fn BlogPost(cx: Scope) -> Element {
    let route = use_route(&cx)?;
    let post = route.parameters.get("post")?;

    cx.render(rsx! {
        div {
            h1 { "Reading blog post: {post}" }
            p { "example blog post" }
        }
    })
}

fn RouteNotFound(cx: Scope) -> Element {
    cx.render(rsx! {
        "Error 404: Route Not Found"
    })
}

fn User(cx: Scope) -> Element {
    let route = use_route(&cx)?;
    let params = route.query_params().unwrap_or_default();

    let name = route.parameters.get("name")?;

    // if bold is specified without content => true
    // if bold is specified => parse, false if invalid
    // default to false
    let bold: bool = params
        .get("bold")
        .and_then(|bold| match bold.is_empty() {
            true => Some(true),
            false => bold.parse().ok(),
        })
        .unwrap_or_default();

    cx.render(rsx! {
        div {
            h1 { "Showing user: {name}" }
            p { "example user content" }

            if bold {
                rsx!{ b { "bold" } }
            } else {
                rsx!{ i { "italic" } }
            }
        }
    })
}

fn UserList(cx: Scope) -> Element {
    cx.render(rsx! {
        "User list"
    })
}
