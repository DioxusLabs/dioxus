use dioxus::prelude::*;
use dioxus_signals::*;

fn main() {
    dioxus_desktop::launch(App);
}

#[component]
fn App(cx: Scope) -> Element {
    let signal = use_signal(cx, || vec![String::from("Hello"), String::from("World")]);

    render! {
        button {
            onclick: move |_| {
                signal.write().push(String::from("Hello"));
            },
            "Add one"
        }
        for i in 0..signal().len() {
            Child { signal: signal.map(move |v| v.get(i).unwrap()) }
        }
    }
}

#[component]
fn Child(cx: Scope, signal: SignalMap<String>) -> Element {
    render! {"{signal:?}"}
}
