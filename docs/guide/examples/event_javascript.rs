#![allow(non_snake_case)]
use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(App);
}

fn App(cx: Scope) -> Element {
    cx.render(rsx! {
        // ANCHOR: rsx
        div {
            onclick: "alert('hello world')",
        }
        // ANCHOR_END: rsx
    })
}
