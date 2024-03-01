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
    let mut vec = use_signal(|| vec![1, 2, 3]);

    let len = vec.len();

    println!("app len: {}", len);
    use_effect(move || {
        println!("app effect len: {}", vec.len());
    });

    rsx! {
        button {
            onclick: move |_| {
                let mut vec = vec.write();
                vec.push(len);
            },
            "Add"
        }
        button {
            onclick: move |_| {
                vec.pop();
            },
            "Remove"
        }
        for i in 0..len {
            Child {
                index: i,
                vec,
            }
        }
    }
}

#[component]
fn Child(index: usize, vec: Signal<Vec<usize>>) -> Element {
    let item = use_memo(move || vec.read()[index]);
    rsx! {
        div { "Item: {item}" }
    }
}
