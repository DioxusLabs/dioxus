#![allow(non_snake_case)]
use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(App);
}

// ANCHOR: component
fn App(cx: Scope) -> Element {
    // count will be initialized to 0 the first time the component is rendered
    let mut count = use_state(cx, || 0);

    cx.render(rsx!(
        h1 { "High-Five counter: {count}" }
        button {
            onclick: move |_| {
                // changing the count will cause the component to re-render
                count += 1
            },
            "Up high!"
        }
        button {
            onclick: move |_| {
                // changing the count will cause the component to re-render
                count -= 1
            },
            "Down low!"
        }
    ))
}
// ANCHOR_END: component
