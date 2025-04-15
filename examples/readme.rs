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

    does_thing();
    match DOES_THING_META {
        Entry::ServerFn { f } => f(),
        Entry::Asset => {}
    }

    rsx! {
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    }
}

#[link_section = concat!("__TEXT,__manganis")]
pub fn does_thing() {
    println!("Hello from the dioxus example!");
}

#[link_section = concat!("__DATA,__manganis")]
pub static DOES_THING_META: Entry = Entry::ServerFn { f: does_thing };

#[repr(C, u8)]
enum Entry {
    ServerFn { f: fn() },
    Asset,
}
