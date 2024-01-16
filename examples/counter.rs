//! Comparison example with leptos' counter example
//! https://github.com/leptos-rs/leptos/blob/main/examples/counters/src/lib.rs

use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app() -> Element {
    let mut counters = use_signal(|| vec![0, 0, 0]);
    let mut sum = use_selector(move || counters.read().iter().copied().sum::<usize>());

    render! {
        div {
            button { onclick: move |_| counters.write().push(0), "Add counter" }
            button { onclick: move |_| { counters.write().pop(); }, "Remove counter" }
            p { "Total: {sum}" }
            for (i, counter) in counters.read().iter().enumerate() {
                li {
                    button { onclick: move |_| counters.write()[i] -= 1, "-1" }
                    input {
                        value: "{counter}",
                        oninput: move |e| {
                            if let Ok(value) = e.value().parse::<usize>() {
                                counters.write()[i] = value;
                            }
                        }
                    }
                    button { onclick: move |_| counters.write()[i] += 1, "+1" }
                    button { onclick: move |_| { counters.write().remove(i); }, "x" }
                }
            }
        }
    }
}
