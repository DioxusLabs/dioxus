//! Comparison example with leptos' counter example
//! https://github.com/leptos-rs/leptos/blob/main/examples/counters/src/lib.rs

use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    let mut counters = use_signal(|| vec![0, 0, 0]);
    let mut sum = use_selector(move || counters.read().iter().copied().sum::<usize>());

    rsx! {
        div {
            button { onclick: move |_| counters.write().push(0), "Add counter" }
            button {
                onclick: move |_| {
                    counters.write().pop();
                },
                "Remove counter"
            }
            p { "Total: {sum}" }
            for i in 0..counters.len() {
                Child { i, counters }
            }
        }
    }
}

#[component]
fn Child(i: usize, counters: Signal<Vec<usize>>) -> Element {
    rsx! {
        li {
            button { onclick: move |_| counters.write()[i] -= 1, "-1" }
            input {
                value: "{counters.read()[i]}",
                oninput: move |e| {
                    if let Ok(value) = e.value().parse::<usize>() {
                        counters.write()[i] = value;
                    }
                }
            }
            button { onclick: move |_| counters.write()[i] += 1, "+1" }
            button {
                onclick: move |_| {
                    counters.write().remove(i);
                },
                "x"
            }
        }
    }
}
