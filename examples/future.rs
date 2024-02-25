//! A simple example that shows how to use the use_future hook to run a background task.
//!
//! use_future assumes your future will never complete - it won't return a value.
//! If you want to return a value, use use_resource instead.

use dioxus::prelude::*;
use std::time::Duration;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);

    // use_future will run the future
    use_future(move || async move {
        loop {
            tokio::time::sleep(Duration::from_millis(200)).await;
            count += 1;
        }
    });

    // We can also spawn futures from effects, handlers, or other futures
    use_effect(move || {
        spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;
            count.set(100);
        });
    });

    rsx! {
        div {
            h1 { "Current count: {count}" }
            button { onclick: move |_| count.set(0), "Reset the count" }
        }
    }
}
