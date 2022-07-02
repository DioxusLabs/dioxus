#![allow(non_snake_case)]
use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(App);
}

// ANCHOR: component
fn App(cx: Scope) -> Element {
    cx.render(rsx! {
        button {
            onclick: move |event| println!("Clicked! Event: {event:?}"),
            "click me!"
        }
    })
}
// ANCHOR_END: component
