#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_router::prelude::*;

fn main() {
    dioxus_desktop::launch(App);
}

fn App(cx: Scope) -> Element {
    use_router(
        &cx,
        &|| RouterConfiguration {
            ..Default::default()
        },
        &|| Segment::content(comp(RootIndex)),
    );

    render! {
        h1 { "hi" }
        Outlet { }
    }
}

fn RootIndex(cx: Scope) -> Element {
    render! {
        h1 { "Simple Example App" }
    }
}
