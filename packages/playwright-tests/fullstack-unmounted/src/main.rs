// Regression test for onunmounted event
// Tests that the onunmounted event fires when an element is removed from the DOM

use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Track whether the element is shown
    let mut show_element = use_signal(|| true);
    // Track whether the unmounted event was triggered
    let mut unmounted_triggered = use_signal(|| false);
    // Track the mounted event for completeness
    let mut mounted_triggered = use_signal(|| false);

    rsx! {
        div {
            id: "status",
            // Show status messages for the test to verify
            if mounted_triggered() {
                span { id: "mounted-status", "Element was mounted." }
            }
            if unmounted_triggered() {
                span { id: "unmounted-status", "The unmounted event was triggered." }
            }
        }

        button {
            id: "toggle-button",
            onclick: move |_| {
                show_element.set(!show_element());
            },
            "Toggle Element"
        }

        if show_element() {
            div {
                id: "test-element",
                onmounted: move |_| {
                    web_sys::console::log_1(&"onmounted fired!".into());
                    mounted_triggered.set(true);
                },
                onunmounted: move |_| {
                    web_sys::console::log_1(&"onunmounted fired!".into());
                    unmounted_triggered.set(true);
                },
                "Lifecycle test element"
            }
        }
    }
}
