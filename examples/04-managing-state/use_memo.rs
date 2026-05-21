//! Deriving values with `use_memo`.
//!
//! `use_memo` caches the result of a closure and re-runs it only when one of the signals
//! it reads changes. It's how you compute values derived from state without doing the work
//! on every render, and without introducing another `use_signal` that you have to keep
//! in sync manually.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut first = use_signal(|| "Ada".to_string());
    let mut last = use_signal(|| "Lovelace".to_string());

    // Recomputes only when `first` or `last` change
    let full_name = use_memo(move || format!("{first} {last}"));

    // Memos compose — derive from other memos or signals
    let initials = use_memo(move || {
        let name = full_name();
        name.split_whitespace()
            .filter_map(|w| w.chars().next())
            .collect::<String>()
    });

    rsx! {
        h1 { "{full_name}" }
        p { "Initials: {initials}" }

        label { "First: " }
        input { value: "{first}", oninput: move |e| first.set(e.value()) }

        label { "Last: " }
        input { value: "{last}", oninput: move |e| last.set(e.value()) }
    }
}
