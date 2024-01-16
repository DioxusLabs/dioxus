#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_router::prelude::*;

fn main() {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Debug)
        .with_module_level("dioxus", log::LevelFilter::Trace)
        .init()
        .unwrap();
    launch(App);
}

#[component]
fn App() -> Element {
    render! {
        Router::<Route> {}
    }
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
    render! {
        h1 { "Your app here" }
        ul {
            li { Link { to: Route::Home {}, "home" } }
            li { Link { to: Route::BlogList {}, "blog" } }
            li { Link { to: Route::BlogPost { post: "tim".into() }, "tims' blog" } }
            li { Link { to: Route::BlogPost { post: "bill".into() }, "bills' blog" } }
            li { Link { to: Route::BlogPost { post: "james".into() }, "james amazing' blog" } }
        }
        Outlet::<Route> {}
    }
}

#[component]
fn Home() -> Element {
    log::debug!("rendering home {:?}", current_scope_id());
    render! { h1 { "Home" } }
}

#[component]
fn BlogList() -> Element {
    log::debug!("rendering blog list {:?}", current_scope_id());
    render! { div { "Blog List" } }
}

#[component]
fn BlogPost(post: String) -> Element {
    log::debug!("rendering blog post {}", post);

    render! {
        div {
            h3 { "blog post: {post}"  }
            Link { to: Route::BlogList {}, "back to blog list" }
        }
    }
}

#[component]
fn Oranges() -> Element {
    render!("Oranges are not apples!")
}
