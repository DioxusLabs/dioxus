//! Example: README.md showcase
//!
//! The example from the README.md

use dioxus::{events::on::MouseEvent, prelude::*};
use dioxus_html_namespace::{button, div, h1};

fn main() {
    dioxus::web::launch(Example)
}

fn Example(cx: Context<()>) -> VNode {
    let name = use_state(&cx, || "..?");

    let handler = move |e: MouseEvent| e.cl;

    cx.render(rsx! {
        h1 { "Hello, {name}" }
        button { "?", onclick: move |event| name.set("world!")}
        button { "?", onclick: move |_| name.set("Dioxus 🎉")}
    })
}
