//! Example: README.md showcase
//!
//! The example from the README.md.

use dioxus::prelude::*;
use std::time::Duration;

fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let (count, set_count) = use_state(&cx, || 0);

    use_future(&cx, move || {
        let set_count = set_count.to_owned();
        async move {
            loop {
                tokio::time::sleep(Duration::from_millis(1000)).await;
                set_count.modify(|f| f + 1);
            }
        }
    });

    cx.render(rsx! {
        div {
            h1 { "Current count: {count}" }
            button {
                onclick: move |_| set_count(0),
                "Reset the count"
            }
        }
    })
}
