//! Regression test for <https://github.com/DioxusLabs/dioxus/issues/4646>

use dioxus::prelude::*;

fn main() {
    dioxus::launch(|| {
        rsx! {
            Comp {}
            Comp {}
            Button {}
        }
    });
}

#[component]
fn Button() -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        button {
            id: "counter",
            onclick: move |_| {
                count += 1;
            },
            "Count: {count}"
        }
    }
}

#[component]
fn Comp(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>) -> Element {
    rsx! {
        div {
            width: 100,
            div {
                ..attributes,
            }
        }
    }
}
