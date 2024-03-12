//! A few ways of mapping elements into rsx! syntax
//!
//! Rsx allows anything that's an iterator where the output type implements Into<Element>, so you can use any of the following:

use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    rsx!(
        div {
            // Use Map directly to lazily pull elements
            {(0..10).map(|f| rsx! { "{f}" })},

            // Collect into an intermediate collection if necessary, and call into_iter
            {["a", "b", "c", "d", "e", "f"]
                .into_iter()
                .map(|f| rsx! { "{f}" })
                .collect::<Vec<_>>()
                .into_iter()},

            // Use optionals
            {Some(rsx! { "Some" })},

            // use a for loop where the body itself is RSX
            for name in 0..10 {
                div { "{name}" }
            }

            // Or even use an unterminated conditional
            if true {
                "hello world!"
            }
        }
    )
}
