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
    let sig = Signal::global(|| 0);

    let blah = 123;

    let out = rsx! {
        div { "hello world {blah}!" }
    };

    out

    // let mut count = use_signal(|| 0);

    // let out = rsx! {
    //     h1 { "High-Five counter: {count}" }
    //     button { onclick: move |_| count += 1, "Up high!" }
    //     button { onclick: move |_| count -= 1, "Down low!" }
    // }
    // .unwrap();

    // dbg!(out.template.get().name);

    // Some(out)
}

#[test]
fn nested_is() {
    // let out = rsx! {
    //     div { "hhi" }
    //     div {
    //         {rsx! { "hi again!" }},
    //         for i in 0..2 {
    //             "first"
    //             div { "hi {i}" }
    //         }

    //         for i in 0..3 {
    //             "Second"
    //             div { "hi {i}" }
    //         }

    //         if false {
    //             div { "hi again?" }
    //         } else if true {
    //             div { "cool?" }
    //         } else {
    //             div { "nice !" }
    //         }
    //     }
    // }
    // .unwrap();

    // // let out = rsx! { {rsx!{ "hi again!" }} }.unwrap();

    // // dbg!(&out.dynamic_nodes[0]);
    // //     let out = rsx! {
    // //         div { "hhi" }
    // //         div {
    // //             {rsx! { "hi again!" }},
    // //             for i in 0..2 {
    // //                 div { "hi {i}" }
    // //             }

    // //             for i in 0..3 {
    // //                 div { "hi {i}" }
    // //             }
    // //         }
    // //     }
    // //     .unwrap();

    // dbg!(&out.template);
    // dbg!(&out.dynamic_nodes[0]);
    // dbg!(&out.dynamic_nodes[1]);
    // dbg!(&out.dynamic_nodes[2]);
    // dbg!(&out.dynamic_nodes[3]);

    // dbg!(&out.dynamic_nodes[1]);
}

// #[test]
// fn hotreload_segments() {

// }
