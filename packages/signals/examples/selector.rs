#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_signals::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
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

#[inline_props]
fn Child(cx: Scope, signal: ReadOnlySignal<usize>) -> Element {
    render! {
        "{signal}"
    }
}
