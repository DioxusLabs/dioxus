#![allow(non_snake_case)]
use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(App);
}

#[rustfmt::skip]
fn App(cx: Scope) -> Element {
    // ANCHOR: prevent_default
cx.render(rsx! {
    input {
        prevent_default: "oninput",
        prevent_default: "onclick",
    }
})
    // ANCHOR_END: prevent_default
}
