#![allow(non_snake_case)]

use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use dioxus_router::*;

#[test]
fn generates_without_error() {
    let mut app = VirtualDom::new(app);
    app.rebuild();

    let out = dioxus_ssr::render_vdom(&app);

    assert_eq!(out, "<nav>navbar</nav><h1>Home</h1>");
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {
            nav { "navbar" }
            Route { to: "/home", Home {} }
        }
    })
}

fn Home(cx: Scope) -> Element {
    cx.render(rsx! { h1 { "Home" } })
}
