//! Example: README.md showcase
//!
//! The example from the README.md.

use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let mut count = use_state(&cx, || 0);

    let opt: Option<u64> = None;

    cx.render(rsx! {
        h1 {
            "node"?: opt,
            "High-Five counter: {count}"
        }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    })
}
