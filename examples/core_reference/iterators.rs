//! Example: Iterators
//! ------------------
//!
//! This example demonstrates how to use iterators with Dioxus.
//! Iterators must be used through the curly braces item in element bodies.
//! While you might be inclined to `.collect::<>` into Html, Dioxus prefers you provide an iterator that
//! resolves to VNodes. It's more efficient and easier to write than having to `collect` everywhere.
//!
//! This also makes it easy to write "pull"-style iterators that don't have a known size.
//!
//! However, when the size of an iterator needs to be known for display purposes, collecting is fine.

use dioxus::prelude::*;

pub static Example: Component = |cx| {
    let example_data = use_state(&cx, || 0);

    let v = (0..10).map(|f| {
        rsx! {
            li { onclick: move |_| example_data.set(f)
                "ID: {f}"
                ul {
                    (0..10).map(|k| rsx!{
                        li {
                            "Sub iterator: {f}.{k}"
                        }
                    })
                }
            }
        }
    });

    cx.render(rsx! {
        h3 {"Selected: {example_data}"}
        ul {
            {v}
        }
    })
};
