#![deny(deprecated)]

use dioxus::prelude::*;

#[component]
fn Item(id: usize) -> Element {
    rsx! { "{id}" }
}

#[component]
fn Wrapper(children: Element) -> Element {
    rsx! { {children} }
}

fn main() {
    let ids = [1, 2, 3];
    let shown = true;
    _ = rsx! {
        ul {
            for id in ids {
                li { key: "{id}" }
            }
            for id in ids {
                Item { key: "{id}", id }
            }
            if shown {
                li { key: "shown-{shown}" }
            } else {
                li { key: "hidden-{shown}" }
            }
            {ids.iter().map(|id| rsx! { li { key: "{id}" } })}
            Wrapper {
                li { key: "{shown}" }
            }
        }
    };
}
