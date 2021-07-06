//! Example: README.md showcase
//!
//! The example from the README.md

use dioxus::prelude::*;
fn main() {
    dioxus::web::launch(Example)
}

fn Example(cx: Context<()>) -> VNode {
    let name = use_state(&cx, || "..?");

    cx.render(rsx! {
        h1 { "Hello, {name}" }
        button { "?", onclick: move |_| name.set("world!")}
        button { "?", onclick: move |_| name.set("Dioxus 🎉")}
    })
}
