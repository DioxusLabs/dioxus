//! Scroll elements using their MountedData
//!
//! Dioxus exposes a few helpful APIs around elements (mimicking the DOM APIs) to allow you to interact with elements
//! across the renderers. This includes scrolling, reading dimensions, and more.
//!
//! In this example we demonstrate how to scroll to a given y offset of the scrollable parent using the `scroll` method on the `MountedData`

use dioxus::html::geometry::PixelsVector2D;
use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    rsx! {
        ScrollToCoordinates {}
        ScrollToCoordinates {}
    }
}

#[component]
fn ScrollToCoordinates() -> Element {
    let mut element = use_signal(|| None);

    rsx! {
        div { border: "1px solid black", position: "relative",

            div {
                height: "300px",
                overflow_y: "auto",

                onmounted: move |event| element.set(Some(event.data())),

                for i in 0..100 {
                    div { height: "20px", "Item {i}" }
                }
            }

            div { position: "absolute", top: 0, right: 0,
                input {
                    r#type: "number",
                    min: "0",
                    max: "99",
                    oninput: move |event| async move {
                        if let Some(ul) = element.cloned() {
                            let data = event.data();
                            if let Ok(value) = data.parsed::<f64>() {
                                ul.scroll(PixelsVector2D::new(0.0, 20.0 * value), ScrollBehavior::Smooth)
                                    .await
                                    .unwrap();
                            }
                        }
                    },
                }
            }
        }
    }
}
