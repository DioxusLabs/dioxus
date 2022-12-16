#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_router::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    use_router(cx, &|| RouterConfiguration::default(), &|| {
        Segment::content(comp(Home))
            .fixed(
                "users",
                Route::empty()
                    .nested(Segment::content(comp(UserList)).catch_all((comp(User), UserId {}))),
            )
            .fixed(
                "blog",
                Route::empty().nested(
                    Segment::content(comp(BlogList)).catch_all((comp(BlogPost), PostId {})),
                ),
            )
            .fallback(comp(E404))
    });

    cx.render(rsx! {
        ul {
            li { Link { target: "/", "Go home!" } }
            li { Link { target: "/users", "List all users" } }
            li { Link { target: "/blog", "Blog posts" }}
        }

        Outlet { }
    })
}

fn Home(cx: Scope) -> Element {
    render!(h1 { "Home" })
}

fn BlogList(cx: Scope) -> Element {
    render! {
        h1 { "Blog Posts" }
        ul {
            li { Link { target: "/blog/1", "First blog post" } }
            li { Link { target: "/blog/2", "Second blog post" } }
            li { Link { target: "/blog/3", "Third blog post" } }
        }
    }
}

struct PostId;
fn BlogPost(cx: Scope) -> Element {
    let post = use_route(cx)?.parameter::<PostId>().unwrap();

    cx.render(rsx! {
        div {
            h1 { "Reading blog post: {post}" }
            p { "example blog post" }
        }
    })
}

fn UserList(cx: Scope) -> Element {
    render! {
        h1 { "Users" }
        ul {
            li { Link { target: "/users/bill", "Bill" } }
            li { Link { target: "/users/jeremy", "Jeremy" } }
            li { Link { target: "/users/adrian", "Adrian" } }
        }
    }
}

struct UserId;
fn User(cx: Scope) -> Element {
    let state = use_route(cx)?;

    let user = state.parameter::<UserId>().unwrap();

    let query = state.query.as_ref().map(|q| q.clone()).unwrap_or_default();
    let bold = query.contains("bold") && !query.contains("bold=false");

    cx.render(rsx! {
        div {
            h1 { "Showing user: {user}" }

            if bold {
                rsx!{ b { "bold" } }
            } else {
                rsx!{ i { "italic" } }
            }
        }
    })
}

fn E404(cx: Scope) -> Element {
    render!(h1 { "Error 404 - Page not Found" })
}
