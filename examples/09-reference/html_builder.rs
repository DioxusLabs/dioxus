//! Build HTML with the typed `view` builder API and normal components.
//!
//! This example is intentionally close to the shape `rsx!` should expand to:
//! HTML constructors for tags, generated attribute builders with static values,
//! static text views, generated component props builders, and dynamic slots only
//! for runtime values.

use dioxus::core as dioxus_core;
use dioxus::{
    core::{
        ComponentFunctionExt, Element,
        view::{View, attr_dyn},
    },
    html::{self, EventsExtension, GlobalAttributesExtension},
    prelude::Props,
};

#[derive(Clone, Copy, PartialEq)]
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
    Ok(Dashboard
        .with_props(
            Dashboard
                .builder()
                .title("HTML builder API")
                .metrics(METRICS)
                .build(),
        )
        .into_vnode())
}

#[dioxus::prelude::component]
fn Dashboard(#[props(into)] title: String, metrics: &'static [Metric]) -> Element {
    Ok(html::main()
        .onclick(|event| println!("{event:?}"))
        .class(dioxus::core::static_value!("dashboard"))
        .child(Header.with_props(Header.builder().title(title).total(metrics.len()).build()))
        .child(
            html::section()
                .class(dioxus::core::static_value!("metric-grid"))
                .child(metrics.iter().copied().map(|metric| {
                    MetricCard.with_props(MetricCard.builder().metric(metric).build())
                })),
        )
        .child(
            html::footer()
                .class(dioxus::core::static_value!("note"))
                .child(dioxus::core::static_text!(
                    "Created with html element builders and component props builders."
                )),
        )
        .into_vnode())
}

#[dioxus::prelude::component]
fn Header(#[props(into)] title: String, total: usize) -> Element {
    Ok(html::header()
        .class(dioxus::core::static_value!("intro"))
        .child(
            html::p()
                .class(dioxus::core::static_value!("eyebrow"))
                .child(dioxus::core::static_text!("typed views")),
        )
        .child(html::h1().child(title))
        .child(html::p().child(format!(
            "{total} cards are rendered from a runtime iterator."
        )))
        .into_vnode())
}

#[dioxus::prelude::component]
fn MetricCard(metric: Metric) -> Element {
    Ok(html::article()
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
        .into_vnode())
}
