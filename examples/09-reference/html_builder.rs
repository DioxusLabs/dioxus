//! Build HTML with typed element constructors and normal components.
//!
//! The builder API is useful when Rust control flow or helper functions are a
//! better fit than an `rsx!` block. It supports the same elements, attributes,
//! events, and child values you would use in regular Dioxus components.

use dioxus::{dioxus_core::view::ViewExt, html, prelude::*};

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

#[component]
fn Dashboard(#[props(into)] title: String, metrics: &'static [Metric]) -> Element {
    Ok(html::main()
        .onclick(|event| println!("{event:?}"))
        .class("dashboard")
        .child(Header.with_props(Header.builder().title(title).total(metrics.len()).build()))
        .child(
            html::section().class("metric-grid").child(
                metrics.iter().copied().map(|metric| {
                    MetricCard.with_props(MetricCard.builder().metric(metric).build())
                }),
            ),
        )
        .child(
            html::footer()
                .class("note")
                .child("Created with typed HTML builders and component props builders."),
        )
        .into_vnode())
}

#[component]
fn Header(#[props(into)] title: String, total: usize) -> Element {
    Ok(html::header()
        .class("intro")
        .child(html::p().class("eyebrow").child("typed views"))
        .child(html::h1().child(title))
        .child(html::p().child(format!(
            "{total} cards are rendered from a runtime iterator."
        )))
        .into_vnode())
}

#[component]
fn MetricCard(metric: Metric) -> Element {
    Ok(html::article()
        .class("metric-card")
        .child(html::span().class("metric-label").child(metric.label))
        .child(html::strong().child(metric.value))
        .child(
            html::small()
                .class("metric-status")
                .child(format!("status: {}", metric.status)),
        )
        .into_vnode())
}
