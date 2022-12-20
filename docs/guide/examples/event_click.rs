#![allow(non_snake_case)]
use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(App);
}

#[rustfmt::skip]
fn App(cx: Scope) -> Element {
    // ANCHOR: rsx
cx.render(rsx! {
    button {
        onclick: move |event| println!("Clicked! Event: {event:?}"),
        "click me!"
    }
})
    // ANCHOR_END: rsx
}
