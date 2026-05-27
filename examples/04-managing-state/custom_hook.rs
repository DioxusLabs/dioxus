//! Writing your own hook.
//!
//! A hook is just a function that calls other hooks. By convention it starts with `use_`
//! and returns whatever state handle the caller needs. Extracting hooks is how you reuse
//! stateful logic across components — like a counter, a timer, or a data fetcher.
//!
//! Hooks rely on being called in the same order every render, so never call them inside
//! `if`, `match`, or loops.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    // Each call to our custom hook produces an independent counter
    let mut counter_a = use_counter(0);
    let mut counter_b = use_counter(100);

    // And a more complex hook that exposes multiple actions
    let (name, set_name, reverse) = use_reversible_name("Ada");

    rsx! {
        h2 { "Two counters from one hook" }
        p { "A: {counter_a}" }
        button { onclick: move |_| counter_a += 1, "Increment A" }

        p { "B: {counter_b}" }
        button { onclick: move |_| counter_b += 1, "Increment B" }

        h2 { "Reversible name" }
        input {
            value: "{name}",
            oninput: move |evt| set_name.call(evt.value()),
        }
        button { onclick: move |_| reverse.call(()), "Reverse" }
        p { "Name: {name}" }
    }
}

// A minimal hook — returns a signal initialized with a value
fn use_counter(initial: i32) -> Signal<i32> {
    use_signal(|| initial)
}

// Hooks can return any combination of signals and callbacks
fn use_reversible_name(initial: &'static str) -> (Signal<String>, Callback<String>, Callback<()>) {
    let mut name = use_signal(|| initial.to_string());

    // `use_callback` memoizes a closure so children don't think it changed every render
    let set_name = use_callback(move |value: String| name.set(value));
    let reverse = use_callback(move |_| {
        let reversed = name.read().chars().rev().collect::<String>();
        name.set(reversed);
    });

    (name, set_name, reverse)
}
