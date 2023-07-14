// Given an `rsx!` invocation with a missing trailing comma,
// ensure the stderr output has an informative span.

use dioxus::prelude::*;

fn main() {
    rsx! {
        p {
            class: "foo bar"
            "Hello world"
        }
    };
}
