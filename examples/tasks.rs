//! Example: README.md showcase
//!
//! The example from the README.md.

use dioxus::prelude::*;
use std::time::Duration;

fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let count = use_state(&cx, || 0);

    use_future(&cx, || {
        for_async![count];
        async move {
            while let _ = tokio::time::sleep(Duration::from_millis(1000)).await {
                *count.modify() += 1;
            }
        }
    });

    cx.render(rsx! {
        div {
            h1 { "High-Five counter: {count}" }
            button {
                onclick: move |_| *count.modify() += 1,
                "Click me!"
            }
        }
    })
}
