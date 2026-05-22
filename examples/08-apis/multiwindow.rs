//! Multiwindow example
//!
//! This example shows how to render multiple desktop windows from one Dioxus tree.
//! Each `Window` creates a renderer target for its children, while context, signals,
//! and event bubbling stay connected to the parent tree.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut next_window = use_signal(|| 0usize);
    let mut windows = use_signal(Vec::<usize>::new);
    let count = use_signal(|| 0);

    rsx! {
        button {
            onclick: move |_| {
                let id = next_window();
                next_window += 1;
                windows.write().push(id);
            },
            "New Window"
        }

        for id in windows() {
            Window {
                key: "{id}",
                onclose: move |_| {
                    windows.write().retain(|window_id| *window_id != id);
                },
                Popup { id, count }
            }
        }
    }
}

#[component]
fn Popup(id: usize, count: Signal<usize>) -> Element {
    let window = dioxus::desktop::window();

    rsx! {
        div {
            h1 { "Popup Window {id}" }
            p { "Count: {count}" }
            button { onclick: move |_| count += 1, "Increment" }
            button { onclick: move |_| window.close(), "Close Window" }
        }
    }
}
