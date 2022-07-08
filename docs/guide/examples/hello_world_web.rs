#![allow(non_snake_case)]
use dioxus::prelude::*;

fn main() {
    dioxus::web::launch(App);
}

fn App(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            "Hello, world!"
        }
    })
}
