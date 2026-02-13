// Regression test for https://github.com/DioxusLabs/dioxus/pull/3480

use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut mounted = use_signal(|| false);

    rsx! {
        div {
            id: "main",
            div {
                id: "mounted-test",
                onmounted: move |_| {
                    mounted.set(true);
                },
                if mounted() {
                    "The mounted event was triggered."
                }
            }

            CleanupTest {}
        }
    }
}

#[component]
fn CleanupTest() -> Element {
    let mut cleanup_triggered = use_signal(|| false);
    let mut show_cleanup_element = use_signal(|| true);

    rsx! {
        // Cleanup test section
        div {
            id: "cleanup-status",
            if cleanup_triggered() {
                span { id: "cleanup-triggered", "Cleanup was called." }
            }
        }

        button {
            id: "toggle-cleanup-element",
            onclick: move |_| {
                show_cleanup_element.set(!show_cleanup_element());
            },
            "Toggle Cleanup Element"
        }

        if show_cleanup_element() {
            div {
                id: "cleanup-test-element",
                onmounted: move |_| {
                    move || { cleanup_triggered.set(true); }
                },
                "Element with cleanup"
            }
        }
    }
}
