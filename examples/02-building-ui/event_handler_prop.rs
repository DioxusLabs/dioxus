//! Passing event handlers as props.
//!
//! Components can accept callbacks using `EventHandler<T>`, which is Dioxus's equivalent
//! of React's `onClick` prop or a closure. Call the handler with `.call(value)` from the
//! child; the parent decides what happens when the event fires.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut log = use_signal(Vec::<String>::new);

    rsx! {
        h1 { "Custom event handlers" }

        // The parent owns the handler — the child just fires it
        FancyButton {
            label: "Say hi",
            onpress: move |name: String| log.write().push(format!("Hi, {name}!")),
        }
        FancyButton {
            label: "Say bye",
            onpress: move |name: String| log.write().push(format!("Bye, {name}!")),
        }

        ul {
            for entry in log.iter() {
                li { "{entry}" }
            }
        }
    }
}

#[component]
fn FancyButton(label: String, onpress: EventHandler<String>) -> Element {
    rsx! {
        button {
            onclick: move |_| onpress.call(label.clone()),
            "{label}"
        }
    }
}
