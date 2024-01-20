use dioxus::prelude::*;
use dioxus_signals::*;

fn main() {
    // dioxus::desktop::launch(App);
}

#[component]
fn App() -> Element {
    let mut signal = use_signal(|| 0);
    let doubled = use_selector(move || signal * 2);

    rsx! {
        button {
            onclick: move |_| signal += 1,
            "Increase"
        }
        Child { signal: doubled }
    }
}

#[component]
fn Child(signal: ReadOnlySignal<usize>) -> Element {
    rsx! { "{signal}" }
}
