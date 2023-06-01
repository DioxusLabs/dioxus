#![allow(non_snake_case)]
use dioxus::prelude::*;
use dioxus_router::prelude::*;

// ANCHOR: outlet
#[derive(Routable, Clone)]
#[rustfmt::skip]
enum Route {
    #[layout(Wrapper)]
        #[route("/")]
        Index {},
}

#[inline_props]
fn Wrapper(cx: Scope) -> Element {
    render! {
        header { "header" }
        // The index route will be rendered here
        Outlet { }
        footer { "footer" }
    }
}

#[inline_props]
fn Index(cx: Scope) -> Element {
    render! {
        h1 { "Index" }
    }
}
// ANCHOR_END: outlet

fn App(cx: Scope) -> Element {
    render! {
        Router {}
    }
}

fn main() {
    let mut vdom = VirtualDom::new(App);
    let _ = vdom.rebuild();
    let html = dioxus_ssr::render(&vdom);
    assert_eq!(
        html,
        "<header>header</header><h1>Index</h1><footer>footer</footer>"
    );
}
