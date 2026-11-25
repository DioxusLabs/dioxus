use dioxus::prelude::*;
use std::cell::Cell;

// Regression tests for https://github.com/DioxusLabs/dioxus/issues/6107.
#[test]
fn moving_formatted_text() {
    let value = Some("data".to_string());
    assert_eq!(
        dioxus_ssr::render_element(rsx! { "{value.unwrap_or_default()}" }),
        "data"
    );
}

#[test]
fn moving_formatted_attribute() {
    let value = Some("data".to_string());
    assert_eq!(
        dioxus_ssr::render_element(rsx! { div { class: "{value.unwrap_or_default()}" } }),
        "<div class=\"data\"></div>"
    );
}

#[test]
fn moving_formatted_key() {
    let value = Some("data".to_string());
    let node = rsx! { div { key: "{value.unwrap_or_default()}" } }.unwrap();
    assert_eq!(node.key(), Some("data"));
}

#[test]
fn moving_formatted_component_property() {
    #[component]
    fn Child(label: String) -> Element {
        rsx! { "{label}" }
    }

    let value = Some("data".to_string());
    assert_eq!(
        dioxus_ssr::render_element(rsx! { Child { label: "{value.unwrap_or_default()}" } }),
        "data"
    );
}

#[test]
fn formatted_text_and_attribute_evaluate_each_segment_once() {
    let calls = Cell::new(0);
    let next = || {
        calls.set(calls.get() + 1);
        "data"
    };
    let html = dioxus_ssr::render_element(rsx! {
        div { class: "prefix-{next()}-suffix", "before {next()} / {next()} after" }
    });
    assert_eq!(html, "<div class=\"prefix-data-suffix\">before data / data after</div>");
    assert_eq!(calls.get(), 3);
}

#[test]
fn formatted_segments_preserve_format_specifiers() {
    let value = 42;
    let html = dioxus_ssr::render_element(rsx! {
        div { class: "value-{value:>5}", "[{value:>5}] [{value:?}]" }
    });
    assert_eq!(
        html,
        format!("<div class=\"value-{value:>5}\">[{value:>5}] [{value:?}]</div>")
    );
}
