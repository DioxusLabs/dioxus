//! Running side effects when state changes.
//!
//! `use_effect` runs a closure after every render where one of the signals it reads has
//! changed. It's the escape hatch for talking to code outside of Dioxus — logging, syncing
//! to `localStorage`, updating the `document.title`, tweaking imperative APIs, and so on.
//!
//! For pure derivations of other signals, reach for `use_memo` instead — effects are meant
//! for side effects, not for computing values used in the UI.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);
    let mut name = use_signal(|| "world".to_string());

    // Runs after every render where `count` changes — use this for side effects
    use_effect(move || {
        println!("count is now {count}");
    });

    // An effect can read multiple signals. It re-runs whenever any of them change.
    use_effect(move || {
        println!("greeting changed: hello, {name} — count is {count}");
    });

    rsx! {
        h1 { "Hello, {name}! ({count})" }

        button { onclick: move |_| count += 1, "Increment count" }
        input {
            value: "{name}",
            oninput: move |evt| name.set(evt.value()),
        }
        p { "Check your terminal — each change fires an effect." }
    }
}
