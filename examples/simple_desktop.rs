#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_router::prelude::*;

fn main() {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Debug)
        .with_module_level("dioxus", log::LevelFilter::Trace)
        .init()
        .unwrap();
    dioxus_desktop::launch(App);
}

#[component]
fn App(cx: Scope) -> Element {
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
fn NavBar(cx: Scope) -> Element {
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
fn Home(cx: Scope) -> Element {
    log::debug!("rendering home {:?}", cx.scope_id());
    render! { h1 { "Home" } }
}

#[component]
fn BlogList(cx: Scope) -> Element {
    log::debug!("rendering blog list {:?}", cx.scope_id());
    render! { div { "Blog List" } }
}

#[component]
fn BlogPost(cx: Scope, post: String) -> Element {
    log::debug!("rendering blog post {}", post);

    render! {
        div {
            h3 { "blog post: {post}"  }
            Link { to: Route::BlogList {}, "back to blog list" }
        }
    }
}

#[component]
fn Oranges(cx: Scope) -> Element {
    render!("Oranges are not apples!")
}
