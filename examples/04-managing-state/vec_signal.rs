//! Signals that hold a collection.
//!
//! `Signal<Vec<T>>` is the workhorse for dynamic lists. Dioxus exposes `push`, `pop`,
//! `remove`, `clear`, and `retain` directly on the signal so you don't have to call
//! `.write()` for the common cases. For anything else, `.write()` returns a guard that
//! derefs to the underlying `Vec`.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut todos = use_signal(|| vec!["learn dioxus".to_string(), "build an app".to_string()]);
    let mut draft = use_signal(String::new);

    rsx! {
        h1 { "Todo list" }

        input {
            placeholder: "New todo...",
            value: "{draft}",
            oninput: move |evt| draft.set(evt.value()),
        }
        button {
            onclick: move |_| {
                if !draft.read().is_empty() {
                    todos.push(draft());
                    draft.set(String::new());
                }
            },
            "Add"
        }

        ul {
            for (index, todo) in todos.read().iter().enumerate() {
                li { key: "{index}",
                    "{todo} "
                    button {
                        onclick: move |_| { todos.remove(index); },
                        "×"
                    }
                }
            }
        }

        button { onclick: move |_| todos.clear(), "Clear all" }
        button {
            // `.retain` accepts a predicate — here we drop any todo containing "x"
            onclick: move |_| todos.retain(|t| !t.contains('x')),
            "Drop todos with 'x'"
        }
        button {
            // For anything custom, `.write()` gives a mutable guard to the Vec
            onclick: move |_| todos.write().sort(),
            "Sort alphabetically"
        }

        p { "{todos.len()} todo(s)" }
    }
}
