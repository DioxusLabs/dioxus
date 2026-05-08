//! The classic counter — the "hello world" of reactive UI.
//!
//! `use_signal` creates a reactive value. Whenever the signal changes, any part of the UI
//! that reads it will re-render automatically. Signals are copy-cheap, so you can freely
//! move them into event handlers and closures.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        h1 { "Count: {count}" }
        button { onclick: move |_| count += 1, "Increment" }
        button { onclick: move |_| count -= 1, "Decrement" }
        button { onclick: move |_| count.set(0), "Reset" }
    }
}
