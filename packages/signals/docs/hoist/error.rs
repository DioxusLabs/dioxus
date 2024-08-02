#[component]
fn Counters() -> Element {
    let counts = use_signal(Vec::new);
    let mut children = use_signal(|| 0);

    rsx! {
        button { onclick: move |_| children += 1, "Add child" }
        button { onclick: move |_| children -= 1, "Remove child" }
        // A signal from a child is read or written to in a parent scope
        "{counts:?}"
        for _ in 0..children() {
            Counter {
                counts
            }
        }
    }
}

#[component]
fn Counter(mut counts: Signal<Vec<Signal<i32>>>) -> Element {
    let mut signal_owned_by_child = use_signal(|| 0);
    // Moving the signal up to the parent may cause issues if you read the signal after the child scope is dropped
    use_hook(|| counts.push(signal_owned_by_child));

    rsx! {
        button {
            onclick: move |_| signal_owned_by_child += 1,
            "{signal_owned_by_child}"
        }
    }
}
