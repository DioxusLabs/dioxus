//! The example from the readme!
//!
//! This example demonstrates how to create a simple counter app with dioxus. The `Signal` type wraps inner values,
//! making them `Copy`, allowing them to be freely used in closures and async functions. `Signal` also provides
//! helper methods like AddAssign, SubAssign, toggle, etc, to make it easy to update the value without running
//! into lock issues.

use dioxus::prelude::*;

fn main() {
    launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);

    let out = rsx! {
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    }
    .unwrap();

    dbg!(out.template.get().name);

    Some(out)
}

#[test]
fn nested_is() {
    let out = rsx! {
        div { "hhi" }
        div {
            for i in 0..2 {
                div { "hi {i}" }
            }

            for i in 0..3 {
                div { "hi {i}" }
            }
        }
    }
    .unwrap();

    dbg!(out.template.get().name);

    dbg!(&out.dynamic_nodes[0]);
    dbg!(&out.dynamic_nodes[1]);
}
