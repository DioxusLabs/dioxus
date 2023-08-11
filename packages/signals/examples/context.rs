#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_signals::Signal;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    // Because signal is never read in this component, this component will not rerun when the signal changes
    use_context_provider(cx, || Signal::new(0));

    render! {
        Child {}
    }
}

fn Child(cx: Scope) -> Element {
    let signal: Signal<i32> = *use_context(cx).unwrap();
    // This component does read from the signal, so when the signal changes it will rerun
    render! {
        "{signal}"
    }
}
