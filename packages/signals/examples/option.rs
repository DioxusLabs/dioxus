use dioxus::prelude::*;
use dioxus_signals::*;

fn main() {
    dioxus_desktop::launch(App);
}

#[component]
fn App(cx: Scope) -> Element {
    let signal = use_signal(cx, || Some(String::from("Hello")));

    render! {
        button {
            onclick: move |_| {
                let new_value = (!signal.read().is_some()).then(|| String::from("Hello"));
                signal.set(new_value);
            },
            "Swap"
        }
        button {
            onclick: move |_| {
                if let Some(value) = &mut *signal.write() {
                    value.push_str(" World");
                }
            },
            "Change"
        }
        if let Some(item) = signal.as_mapped_ref(){
            render! {
                Child { signal: item }
            }
        }
    }
}

#[component]
fn Child(cx: Scope, signal: MappedSignal<String>) -> Element {
    render! {"{signal:?}"}
}
