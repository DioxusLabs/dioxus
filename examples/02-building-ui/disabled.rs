//! A simple demonstration of how to set attributes on buttons to disable them.
//!
//! This example also showcases the shorthand syntax for attributes, and how signals themselves implement IntoAttribute

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut disabled = use_signal(|| false);

    rsx! {
        div { text_align: "center", margin: "20px", display: "flex", flex_direction: "column", align_items: "center",
            button {
                onclick: move |_| disabled.toggle(),
                "click to "
                if disabled() { "enable" } else { "disable" }
                " the lower button"
            }
            button { disabled, "lower button" }
        }
    }
}
