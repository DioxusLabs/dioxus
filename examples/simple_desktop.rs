#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus::router::prelude::*;

fn main() {
    launch_desktop(|| {
        rsx! {
            Router::<Route> {}
        }
    });
}

#[derive(Routable, Clone)]
#[rustfmt::skip]
enum Route {
    #[layout(NavBar)]
        #[route("/")]
        Home {},
        #[nest("/new")]
            #[route("/")]
            BlogList {},
            #[route("/:post")]
            BlogPost {
                post: String,
            },
        #[end_nest]
        #[route("/oranges")]
        Oranges {},
}

#[component]
fn NavBar() -> Element {
    rsx! {
        h1 { "Your app here" }
        ul {
            li {
                Link { to: Route::Home {}, "home" }
            }
            li {
                Link { to: Route::BlogList {}, "blog" }
            }
            li {
                Link {
                    to: Route::BlogPost {
                        post: "tim".into(),
                    },
                    "tims' blog"
                }
            }
            li {
                Link {
                    to: Route::BlogPost {
                        post: "bill".into(),
                    },
                    "bills' blog"
                }
            }
            li {
                Link {
                    to: Route::BlogPost {
                        post: "james".into(),
                    },
                    "james amazing' blog"
                }
            }
        }
        Outlet::<Route> {}
    }
}

#[component]
fn Home() -> Element {
    println!("rendering home {:?}", current_scope_id());
    rsx! { h1 { "Home" } }
}

#[component]
fn BlogList() -> Element {
    println!("rendering blog list {:?}", current_scope_id());
    rsx! { div { "Blog List" } }
}

#[component]
fn BlogPost(post: String) -> Element {
    println!("rendering blog post {}", post);

    rsx! {
        div {
            h3 { "blog post: {post}" }
            Link { to: Route::BlogList {}, "back to blog list" }
        }
    }
}

#[component]
fn Oranges() -> Element {
    rsx!("Oranges are not apples!")
}
