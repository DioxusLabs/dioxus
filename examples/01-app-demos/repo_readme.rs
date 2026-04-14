//! The example from the readme!
//!
//! This example demonstrates how to create a simple counter app with dioxus. The `Signal` type wraps inner values,
//! making them `Copy`, allowing them to be freely used in closures and async functions. `Signal` also provides
//! helper methods like AddAssign, SubAssign, toggle, etc, to make it easy to update the value without running
//! into lock issues.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);

    // fails because private is not in scope
    <W as A>::private::A();

    rsx! {
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    }
}

pub enum MyPhantom<T> {
    A,
    B(T),
}

mod private {
    use crate::{A, MyPhantom, W};
    use std::ops::Deref;

    // Works because private is in scope
    fn test() {
        <W as A>::private::A();
    }

    struct Private;

    impl Deref for MyPhantom<Private> {
        type Target = fn();

        fn deref(&self) -> &Self::Target {
            fn f() {
                println!("Hello from the private function!");
            }
            &(f as fn())
        }
    }
}

pub struct W;

pub trait A {
    type private<T>;
}

impl A for W {
    type private<T> = MyPhantom<T>;
}
