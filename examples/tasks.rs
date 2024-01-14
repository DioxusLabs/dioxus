//! Example: README.md showcase
//!
//! The example from the README.md.

use dioxus::prelude::*;
use std::time::Duration;

fn main() {
    dioxus_desktop::launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);

    use_future(move |_| async move {
        loop {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            count += 1;
        }
    });

    rsx! {
        div {
            h1 { "Current count: {count}" }
            button {
                onclick: move |_| count.set(0),
                "Reset the count"
            }
        }
    }
}
