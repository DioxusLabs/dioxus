// Regression test for https://github.com/DioxusLabs/dioxus/pull/3958

use dioxus::prelude::*;

fn main() {
    launch(|| {
        rsx! {
            "Hello World!"
        }
    });
}
