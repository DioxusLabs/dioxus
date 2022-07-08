#![allow(non_snake_case)]
use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(App);
}

fn App(cx: Scope) -> Element {
    // ANCHOR: rsx
    cx.render(rsx! {
        div {
            onclick: move |_event| {},
            "outer",
            button {
                onclick: move |event| {
                    // now, outer won't be triggered
                    event.cancel_bubble();
                },
                "inner"
            }
        }
    })
    // ANCHOR_END: rsx
}
