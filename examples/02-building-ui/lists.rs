//! Rendering lists.
//!
//! Use a `for` loop inside `rsx!` to render a list from an iterator. When list items can
//! be reordered, added, or removed, provide a stable `key` so Dioxus can efficiently
//! reuse existing elements rather than re-rendering them all.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut items = use_signal(|| {
        vec![
            "apple".to_string(),
            "banana".to_string(),
            "cherry".to_string(),
        ]
    });
    let mut next_id = use_signal(|| 0);

    rsx! {
        h1 { "Grocery list" }

        button {
            onclick: move |_| {
                let n = next_id();
                next_id += 1;
                items.push(format!("item {n}"));
            },
            "Add item"
        }
        button {
            onclick: move |_| { items.pop(); },
            "Remove last"
        }

        ul {
            // The `key` is a stable identifier used to match up items across renders.
            // Here we use the item's text, but in real apps prefer a database id.
            for item in items.iter() {
                li { key: "{item}", "{item}" }
            }
        }

        // You can also use iterator adapters with .map() for anything more complex than a for loop
        p { "Uppercased:" }
        ul {
            {items.iter().map(|item| rsx! { li { "{item.to_uppercase()}" } })}
        }
    }
}
