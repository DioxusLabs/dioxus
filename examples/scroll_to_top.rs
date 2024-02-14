//! Scroll elements using their MountedData
//!
//! Dioxus exposes a few helpful APIs around elements (mimicking the DOM APIs) to allow you to interact with elements
//! across the renderers. This includes scrolling, reading dimensions, and more.
//!
//! In this example we demonstrate how to scroll to the top of the page using the `scroll_to` method on the `MountedData`

use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    let mut header_element = use_signal(|| None);

    rsx! {
        div {
            h1 {
                onmounted: move |cx| header_element.set(Some(cx.data())),
                "Scroll to top example"
            }

            for i in 0..100 {
                div { "Item {i}" }
            }

            button {
                onclick: move |_| async move {
                    if let Some(header) = header_element.cloned() {
                        let _ = header.scroll_to(ScrollBehavior::Smooth).await;
                    }
                },
                "Scroll to top"
            }
        }
    }
}
