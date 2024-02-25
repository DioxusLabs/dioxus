//! The example from the readme!
//!
//! This example demonstrates how to create a simple counter app with dioxus. The `Signal` type wraps inner values,
//! making them `Copy`, allowing them to be freely used in closures and and async functions. `Signal` also provides
//! helper methods like AddAssign, SubAssign, toggle, etc, to make it easy to update the value without running
//! into lock issues.

use dioxus::prelude::*;

fn main() {
    launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    }
}
