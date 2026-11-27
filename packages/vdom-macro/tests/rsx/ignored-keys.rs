#![deny(deprecated)]

use dioxus::prelude::*;

#[component]
fn Item(id: usize) -> Element {
    rsx! { "{id}" }
}

fn main() {
    let ids = [1, 2, 3];
    let id = 4;
    _ = rsx! {
        ul {
            li { key: "{id}" }
            Item { key: "{id}", id }
        }
        for id in ids {
            div {
                span { key: "{id}" }
            }
        }
    };
}
