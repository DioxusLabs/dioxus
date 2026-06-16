//! Define a typed custom element namespace for rsx!.
//!
//! The rsx! macro lowers identifier elements like `analyticsPanel {}` into a
//! call to an in-scope `analyticsPanel()` constructor. Attribute identifiers
//! lower into builder methods, so custom typed attributes are extension methods.

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
