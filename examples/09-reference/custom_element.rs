//! Define a typed custom element namespace for rsx!.
//!
//! The rsx! macro lowers identifier elements like `analytics_panel {}` into a
//! call to an in-scope `analytics_panel()` constructor. Attribute identifiers
//! lower into builder methods, so custom typed attributes are extension methods.

use dioxus::{
    core::view::{
        el, AttributeDescriptor, AttributeTarget, El, IntoAttributeBuilderValue, TagName,
    },
    prelude::*,
};

pub struct AnalyticsPanel;

impl TagName for AnalyticsPanel {
    const NAME: &'static str = "analytics-panel";
}

pub const fn analytics_panel() -> El<AnalyticsPanel, (), ()> {
    el::<AnalyticsPanel>()
}

impl<Attrs, Children> GlobalAttributesExtension for El<AnalyticsPanel, Attrs, Children> {}

pub struct MetricAttribute;

impl AttributeDescriptor for MetricAttribute {
    const NAME: &'static str = "metric";
}

pub trait AnalyticsPanelExtension: AttributeTarget + Sized {
    fn metric<Marker, Value>(
        self,
        value: Value,
    ) -> <Value as IntoAttributeBuilderValue<Self, MetricAttribute, Marker>>::Output
    where
        Value: IntoAttributeBuilderValue<Self, MetricAttribute, Marker>,
    {
        <Value as IntoAttributeBuilderValue<Self, MetricAttribute, Marker>>::append_to(value, self)
    }
}

impl<Attrs, Children> AnalyticsPanelExtension for El<AnalyticsPanel, Attrs, Children> {}

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let selected_metric = "conversion-rate";

    rsx! {
        analytics_panel {
            class: "metric-card",
            metric: selected_metric,
            h2 { "Revenue" }
            p { "Custom elements can still use regular HTML children." }
        }
    }
}
