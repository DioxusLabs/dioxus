//! Adjust the zoom of a desktop app
//!
//! This example shows how to adjust the zoom of a desktop app using the webview.zoom method.

use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    let mut level = use_signal(|| 1.0);

    rsx! {
        h1 { "Zoom level: {level}" }
        p { "Change the zoom level of the webview by typing a number in the input below."}
        input {
            r#type: "number",
            value: "{level}",
            oninput: move |e| {
                if let Ok(new_zoom) = e.value().parse::<f64>() {
                    level.set(new_zoom);
                    dioxus::desktop::window().webview.zoom(new_zoom);
                }
            }
        }
    }
}
