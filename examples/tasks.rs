//! Example: README.md showcase
//!
//! The example from the README.md.

use std::time::Duration;

use dioxus::prelude::*;
fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope<()>) -> Element {
    let mut count = use_state(&cx, || 0);

    cx.push_task(|| async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        count += 1;
    });

    cx.render(rsx! {
        div {
            h1 { "High-Five counter: {count}" }
            button {
                onclick: move |_| count +=1 ,
                "Click me!"
            }
        }
    })
}
