#[component]
fn Counters() -> Element {
    let mut counts = use_signal(Vec::new);

    rsx! {
        button { onclick: move |_| counts.write().push(0), "Add child" }
        button {
            onclick: move |_| {
                counts.write().pop();
            },
            "Remove child"
        }
        "{counts:?}"
        // Instead of passing up a signal, we can just write to the signal that lives in the parent
        for index in 0..counts.len() {
            Counter {
                index,
                counts
            }
        }
    }
}

#[component]
fn Counter(index: usize, mut counts: Signal<Vec<i32>>) -> Element {
    rsx! {
        button {
            onclick: move |_| counts.write()[index] += 1,
            "{counts.read()[index]}"
        }
    }
}
