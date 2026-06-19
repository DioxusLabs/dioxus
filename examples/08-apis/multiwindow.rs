//! Multiwindow example
//!
//! This example shows how to implement a simple multiwindow application using dioxus.
//! This works by rendering a `Window` component when the user clicks a button.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut windows = use_signal(Vec::<usize>::new);
    let mut next_id = use_signal(|| 0usize);

    let onclick = move |_| {
        let id = next_id();
        next_id.set(id + 1);
        windows.write().push(id);
    };

    rsx! {
        button { onclick, "New Window" }
        for id in windows() {
            Window {
                key: "{id}",
                onclose: move |_| windows.write().retain(|window_id| *window_id != id),
                Popup {}
            }
        }
    }
}

#[component]
fn Popup() -> Element {
    let mut count = use_signal(|| 0);
    rsx! {
        div {
            h1 { "Popup Window" }
            p { "Count: {count}" }
            button { onclick: move |_| count += 1, "Increment" }
        }
    }
}
