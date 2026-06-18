//! Define a typed custom element for `rsx!`.
//!
//! Use `define_elements!` when your app needs project-specific element names or
//! attributes that should type-check like built-in Dioxus HTML elements.

use dioxus::prelude::*;

dioxus::html::define_elements! {
    #[element(name = "analytics-panel")]
    analyticsPanel {
        metric,
        #[attr(name = "data-region")]
        region,
    }
}

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let selected_metric = "conversion-rate";

    rsx! {
        analyticsPanel {
            class: "metric-card",
            metric: selected_metric,
            region: "north-america",
            h2 { "Revenue" }
            p { "Custom elements can still use regular HTML children." }
        }
    }
}
