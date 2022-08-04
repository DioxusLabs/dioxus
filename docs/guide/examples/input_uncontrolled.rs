#![allow(non_snake_case)]
use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(App);
}

// ANCHOR: component
fn App(cx: Scope) -> Element {
    cx.render(rsx! {
        form {
            onsubmit: move |event| {
                println!("Submitted! {event:?}")
            },
            input { name: "name", },
            input { name: "age", },
            input { name: "date", },
            input { r#type: "submit", },
        }
    })
}
// ANCHOR_END: component
