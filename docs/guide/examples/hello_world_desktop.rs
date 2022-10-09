// ANCHOR: all
#![allow(non_snake_case)]
use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(App);
}

// ANCHOR: component
fn App(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            "Hello, world!"
        }
    })
}
// ANCHOR_END: component
// ANCHOR_END: all
