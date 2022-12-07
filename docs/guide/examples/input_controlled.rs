#![allow(non_snake_case)]
use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(App);
}

// ANCHOR: component
fn App(cx: Scope) -> Element {
    let name = use_state(cx, || "bob".to_string());

    cx.render(rsx! {
        input {
            // we tell the component what to render
            value: "{name}",
            // and what to do when the value changes
            oninput: move |evt| name.set(evt.value.clone()),
        }
    })
}
// ANCHOR_END: component
