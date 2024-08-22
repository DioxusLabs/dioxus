//! A simple counters example that stores a list of items in a vec and then iterates over them.

use dioxus::prelude::*;

const STYLE: Asset = asset!("/examples/assets/counter.css");

fn main() {
    launch(app);
}

fn app() -> Element {
    // Store the counters in a signal
    let mut counters = use_signal(|| vec![0, 0, 0]);

    // Whenever the counters change, sum them up
    let sum = use_memo(move || counters.read().iter().copied().sum::<i32>());

    rsx! {
        document::Stylesheet { href: STYLE }

        div { id: "controls",
            button { onclick: move |_| counters.write().push(0), "Add counter" }
            button { onclick: move |_| { counters.write().pop(); }, "Remove counter" }
        }

        h3 { "Total: {sum}" }

        // Calling `iter` on a Signal<Vec<>> gives you a GenerationalRef to each entry in the vec
        // We enumerate to get the idx of each counter, which we use later to modify the vec
        for (i, counter) in counters.iter().enumerate() {
            // We need a key to uniquely identify each counter. You really shouldn't be using the index, so we're using
            // the counter value itself.
            //
            // If we used the index, and a counter is removed, dioxus would need to re-write the contents of all following
            // counters instead of simply removing the one that was removed
            //
            // You should use a stable identifier for the key, like a unique id or the value of the counter itself
            li { key: "{i}",
                button { onclick: move |_| counters.write()[i] -= 1, "-1" }
                input {
                    r#type: "number",
                    value: "{counter}",
                    oninput: move |e| {
                        if let Ok(value) = e.parsed() {
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
