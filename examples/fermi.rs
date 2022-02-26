#![allow(non_snake_case)]

use dioxus::prelude::*;
use fermi::prelude::*;

fn main() {
    dioxus::desktop::launch(app)
}

static NAME: Atom<String> = |_| "world".to_string();

fn app(cx: Scope) -> Element {
    let name = use_read(&cx, NAME);

    cx.render(rsx! {
        div { "hello {name}!" }
        Child {}
    })
}

fn Child(cx: Scope) -> Element {
    let set_name = use_set(&cx, NAME);

    cx.render(rsx! {
        button {
            onclick: move |_| set_name("dioxus".to_string()),
            "reset name"
        }
    })
}
