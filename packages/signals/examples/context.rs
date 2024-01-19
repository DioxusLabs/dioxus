#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_signals::Signal;

fn main() {
    // dioxus_desktop::launch(app);
}

// Because signal is never read in this component, this component will not rerun when the signal changes
fn app() -> Element {
    use_context_provider(|| Signal::new(0));
    rsx! { Child {} }
}

// This component does read from the signal, so when the signal changes it will rerun
#[component]
fn Child() -> Element {
    let signal: Signal<i32> = use_context();
    rsx! { "{signal}" }
}
