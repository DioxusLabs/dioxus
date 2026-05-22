//! dynamic attributes in dioxus necessitate an allocated node ID.
//!
//! This tests to ensure we clean it up

use dioxus::prelude::*;
use dioxus_core::{ScopeId, generation};
use dioxus_renderer_oracle::RendererOracle;

#[test]
fn attrs_cycle() {
    tracing_subscriber::fmt::init();

    fn app() -> Element {
        match generation() {
            1 => {
                let id = 1;
                rsx! { div { h1 { class: "{id}", id: "{id}" } } }
            }
            3 => {
                let id = 3;
                rsx! { div { h1 { class: "{id}", id: "{id}" } } }
            }
            _ => rsx! { div {} },
        }
    }

    fn expected_1() -> Element {
        rsx! { div { h1 { class: "1", id: "1" } } }
    }

    fn expected_3() -> Element {
        rsx! { div { h1 { class: "3", id: "3" } } }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);

    // Anchor diff: instead of `replaces=1` (placeholder swap), we get
    // `loads=1` + `removes=1` for each empty<->populated cycle. Total
    // mutations match: 2 in both models.
    dom.mark_dirty(ScopeId::APP);
    let summary = oracle.render(&mut dom);
    oracle.assert_matches(expected_1);
    assert_eq!(summary.set_attrs, 2);
    assert_eq!(summary.loads, 1);
    assert_eq!(summary.removes, 1);

    dom.mark_dirty(ScopeId::APP);
    let summary = oracle.render(&mut dom);
    oracle.assert_matches(app);
    assert_eq!(summary.loads, 1);
    assert_eq!(summary.removes, 1);

    dom.mark_dirty(ScopeId::APP);
    let summary = oracle.render(&mut dom);
    oracle.assert_matches(expected_3);
    assert_eq!(summary.set_attrs, 2);
    assert_eq!(summary.loads, 1);
    assert_eq!(summary.removes, 1);

    dom.mark_dirty(ScopeId::APP);
    let summary = oracle.render(&mut dom);
    oracle.assert_matches(app);
    assert_eq!(summary.loads, 1);
    assert_eq!(summary.removes, 1);
}
