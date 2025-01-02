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
            onmounted: move |_| {
                mounted.set(true);
            },
            if mounted() {
                "The mounted event was triggered."
            }
        }
    }
}
