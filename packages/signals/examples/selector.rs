use dioxus::prelude::*;
use dioxus_signals::*;

fn main() {
    dioxus_desktop::launch(App);
}

#[component]
fn App(cx: Scope) -> Element {
    let signal = use_signal(cx, || 0);
    let doubled = use_selector(cx, move || signal * 2);

    render! {
        button {
            onclick: move |_| *signal.write() += 1,
            "Increase"
        }
        Child {
            signal: doubled
        }
    }
}

#[component]
fn Child(cx: Scope, signal: ReadOnlySignal<usize>) -> Element {
    render! {
        "{signal}"
    }
}
