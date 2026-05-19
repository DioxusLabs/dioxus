//! dynamic attributes in dioxus necessitate an allocated node ID.
//!
//! This tests to ensure we clean it up

use dioxus::prelude::*;
use dioxus_renderer_oracle::Sequence;

#[test]
fn attrs_cycle() {
    tracing_subscriber::fmt::init();

    Sequence::new()
        .render(rsx! { div {} })
        .render_with_expected(
            || {
                let id = 1;
                rsx! { div { h1 { class: "{id}", id: "{id}" } } }
            },
            rsx! { div { h1 { class: "1", id: "1" } } },
        )
        .render(rsx! { div {} })
        .render_with_expected(
            || {
                let id = 3;
                rsx! { div { h1 { class: "{id}", id: "{id}" } } }
            },
            rsx! { div { h1 { class: "3", id: "3" } } },
        )
        .render(rsx! { div {} })
        .assert_edit_summary(1, |s| {
            assert_eq!(s.set_attrs, 2);
            assert_eq!(s.replaces, 1);
        })
        .assert_edit_summary(2, |s| assert_eq!(s.replaces, 1))
        .assert_edit_summary(3, |s| {
            assert_eq!(s.set_attrs, 2);
            assert_eq!(s.replaces, 1);
        })
        .assert_edit_summary(4, |s| assert_eq!(s.replaces, 1))
        .run();
}
