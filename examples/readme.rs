//! The example from the readme!
//!
//! This example demonstrates how to create a simple counter app with dioxus. The `Signal` type wraps inner values,
//! making them `Copy`, allowing them to be freely used in closures and async functions. `Signal` also provides
//! helper methods like AddAssign, SubAssign, toggle, etc, to make it easy to update the value without running
//! into lock issues.

use std::future::Future;

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    }
}

// fn LazyLoad() -> Element {
//     use wasm_split::*;

//     fn MyModule(props: ()) -> Element {
//         rsx! {}
//     }

//     dioxus::router::maybe_wasm_split! {
//         if wasm_split {
//             ()
//         } else {
//             ()
//         }
//     };

//     static MODULE: LazyLoader<(), Element> =
//         lazy_loader!(extern "one" fn MyModule(props: ()) -> Element);

//     use_resource(|| async move {}).suspend()?;
//     MODULE.call(()).unwrap()
// }
