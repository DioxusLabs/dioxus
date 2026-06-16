//! Build HTML with the typed `view` builder API.
//!
//! This example is intentionally close to the shape `rsx!` should expand to:
//! HTML constructors for tags, generated attribute builders with static values,
//! static text views, and dynamic slots only for runtime values.

use dioxus::{
    core::{
        Element,
        view::{View, attr_dyn, keyed},
    },
    html::{self, GlobalAttributesExtension},
};

#[derive(Clone, Copy)]
struct Metric {
    label: &'static str,
    value: &'static str,
    status: &'static str,
}

const METRICS: &[Metric] = &[
    Metric {
        label: "Hydration",
        value: "stable",
        status: "ready",
    },
    Metric {
        label: "Templates",
        value: "const",
        status: "cached",
    },
    Metric {
        label: "Diffs",
        value: "typed",
        status: "tracked",
    },
];

fn main() {
    println!("{}", dioxus_ssr::render_element(app()));
}

fn app() -> Element {
    Ok(dashboard("HTML builder API", METRICS).into_vnode())
}

fn dashboard<'a>(title: &'a str, metrics: &'a [Metric]) -> impl View {
    html::main()
        .class(dioxus::core::static_value!("dashboard"))
        .child(header(title, metrics.len()))
        .child(
            html::section()
                .class(dioxus::core::static_value!("metric-grid"))
                .child(
                    metrics
                        .iter()
                        .copied()
                        .map(|metric| keyed(metric_card(metric), metric.label).into_vnode()),
                ),
        )
        .child(
            html::footer()
                .class(dioxus::core::static_value!("note"))
                .child(dioxus::core::static_text!(
                    "Created with html element builders and view::View::into_vnode()."
                )),
        )
}

fn header(title: &str, total: usize) -> impl View {
    html::header()
        .class(dioxus::core::static_value!("intro"))
        .child(
            html::p()
                .class(dioxus::core::static_value!("eyebrow"))
                .child(dioxus::core::static_text!("typed views")),
        )
        .child(html::h1().child(title.to_string()))
        .child(html::p().child(format!(
            "{total} cards are rendered from a runtime iterator."
        )))
}

fn metric_card(metric: Metric) -> impl View {
    html::article()
        .class(dioxus::core::static_value!("metric-card"))
        .attr(attr_dyn("data-status", metric.status, None, false))
        .child(
            html::span()
                .class(dioxus::core::static_value!("metric-label"))
                .child(metric.label),
        )
        .child(html::strong().child(metric.value))
        .child(
            html::small()
                .class(dioxus::core::static_value!("metric-status"))
                .child(format!("status: {}", metric.status)),
        )
}
