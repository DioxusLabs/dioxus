#![allow(non_snake_case)]
use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(App);
}

// ANCHOR: component
fn App(cx: Scope) -> Element {
    let list = use_ref(&cx, Vec::new);
    let list_formatted = format!("{:?}", *list.read());

    cx.render(rsx!(
        p { "Current list: {list_formatted}" }
        button {
            onclick: move |event| {
                list.write().push(event)
            },
            "Click me!"
        }
    ))
}
// ANCHOR_END: component
