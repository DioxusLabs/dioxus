//! A simple example that shows how to use the use_future hook to run a background task.
//!
//! use_future won't return a value, analogous to use_effect.
//! If you want to return a value from a future, use use_resource instead.

use async_std::task::sleep;
use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);

    // use_future will run the future
    use_future(move || async move {
        loop {
            sleep(std::time::Duration::from_millis(200)).await;
            count += 1;
        }
    });

    // We can also spawn futures from effects, handlers, or other futures
    use_effect(move || {
        spawn(async move {
            sleep(std::time::Duration::from_secs(5)).await;
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
