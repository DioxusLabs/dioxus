//! Example: README.md showcase
//!
//! The example from the README.md.

use dioxus::prelude::*;
fn main() {
    dioxus::desktop::launch(App);
}

static App: Component<()> = |cx| {
    let mut count = use_state(&cx, || 0);

    cx.render(rsx! {
        div {
            h1 { "High-Five counter: {count}" }
            button { onclick: move |_| count += 1, "Up high!" }
            button { onclick: move |_| count -= 1, "Down low!" }
        }
    })
};
