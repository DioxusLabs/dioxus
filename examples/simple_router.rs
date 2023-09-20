#![allow(non_snake_case)]
use dioxus::prelude::*;
use dioxus_router::prelude::*;

#[derive(Routable, Clone, PartialEq)]
enum Route {
    #[layout(Nav)]
    #[route("/")]
    Homepage {},

    #[route("/blog/:id")]
    Blog { id: String },
}

#[component]
fn Homepage(cx: Scope) -> Element {
    render! { h1 { "Welcome home" } }
}

#[component]
fn Blog(cx: Scope, id: String) -> Element {
    render! {
        h1 { "How to make: " }
        p { "{id}" }
    }
}

#[component]
fn Nav(cx: Scope) -> Element {
    render! {
        nav {
            li { Link { to: Route::Homepage { }, "Go home" } }
            li { Link { to: Route::Blog { id: "Brownies".to_string() }, "Learn Brownies" } }
            li { Link { to: Route::Blog { id: "Cookies".to_string() }, "Learn Cookies"  } }
        }
        div { Outlet::<Route> {} }
    }
}

fn main() {
    dioxus_desktop::launch(|cx| render!(Router::<Route> {}));
}
