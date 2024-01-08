//! Comparison example with leptos' counter example
//! https://github.com/leptos-rs/leptos/blob/main/examples/counters/src/lib.rs

use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let counters = use_state(cx, || vec![0, 0, 0]);
    let sum: usize = counters.iter().copied().sum();

    render! {
        div {
            button { onclick: move |_| counters.make_mut().push(0), "Add counter" }
            button { onclick: move |_| { counters.make_mut().pop(); }, "Remove counter" }
            p { "Total: {sum}" }
            for (i, counter) in counters.iter().enumerate() {
                li {
                    button { onclick: move |_| counters.make_mut()[i] -= 1, "-1" }
                    input {
                        value: "{counter}",
                        oninput: move |e| {
                            if let Ok(value) = e.value().parse::<usize>() {
                                counters.make_mut()[i] = value;
                            }
                        }
                    }
                    button { onclick: move |_| counters.make_mut()[i] += 1, "+1" }
                    button { onclick: move |_| { counters.make_mut().remove(i); }, "x" }
                }
            }
        }
    }
}
