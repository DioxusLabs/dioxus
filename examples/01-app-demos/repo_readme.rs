//! The example from the readme!
//!
//! This example demonstrates how to create a simple counter app with dioxus. The `Signal` type wraps inner values,
//! making them `Copy`, allowing them to be freely used in closures and async functions. `Signal` also provides
//! helper methods like AddAssign, SubAssign, toggle, etc, to make it easy to update the value without running
//! into lock issues.

use dioxus::prelude::*;

use crate::private::{A, W};

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);

    // fails because private is not in scope
    W.private();

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
    trait Private<T> {}
    impl Private<Sneaky> for W {}
    struct Sneaky;

    pub struct W;

    pub trait A {
        fn public(&self) {}

        fn private<T>(&self)
        where
            Self: Private<T>,
        {
        }
    }

    impl A for W {}

    fn test() {
        W.private();
    }
}
