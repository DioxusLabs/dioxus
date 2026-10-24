//! Sharing state between sibling components by lifting it up.
//!
//! When two components need to read or edit the same value, put the signal in their
//! common parent and pass it down as a prop. This keeps the data flow obvious: the parent
//! owns the state, the children receive a `Signal<T>` handle, and mutations show up
//! everywhere the signal is read.
//!
//! For state that needs to be shared across many layers, reach for `use_context_provider`
//! instead — see `context_api.rs`.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    // The parent owns the signal; both children share it
    let counter = use_signal(|| 0);

    rsx! {
        h1 { "Two views of the same counter" }
        Controls { counter }
        Display { counter }
    }
}

#[component]
fn Controls(mut counter: Signal<i32>) -> Element {
    rsx! {
        button { onclick: move |_| counter += 1, "Increment" }
        button { onclick: move |_| counter -= 1, "Decrement" }
        button { onclick: move |_| counter.set(0), "Reset" }
    }
}

#[component]
fn Display(counter: Signal<i32>) -> Element {
    rsx! {
        p { "Count is: {counter}" }
        p {
            if counter() > 0 { "Positive" }
            else if counter() < 0 { "Negative" }
            else { "Zero" }
        }
    }
}
