use dioxus::prelude::*;
use dioxus_core::{ScopeId, generation};
use dioxus_renderer_oracle::RendererOracle;

#[test]
fn toggle_option_text() {
    fn empty() -> Element {
        let text: Option<&str> = None;
        rsx! {
            div {
                {text}
            }
        }
    }

    fn app() -> Element {
        match generation() {
            1 => rsx! { div { "hello" } },
            _ => empty(),
        }
    }

    fn expected_hello() -> Element {
        rsx! { div { "hello" } }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches(empty);

    // Packed dynamic text slots toggle without placeholder swaps.
    dom.mark_dirty(ScopeId::APP);
    let summary = oracle.render(&mut dom);
    oracle.assert_matches(expected_hello);
    assert_eq!(summary.replaces, 0);

    dom.mark_dirty(ScopeId::APP);
    let summary = oracle.render(&mut dom);
    oracle.assert_matches(empty);
    assert_eq!(summary.replaces, 0);
}

/// Regression: a dynamic node (`if show_b`) placed before a *non-first* static
/// sibling (`span { "2" }`) must land at the correct live position even when an
/// earlier dynamic sibling (`{"A"}`) is already materialized in the DOM.
///
/// The incremental create-then-create path placed the slot with
/// `push_id(root) + child(static_insertion_index) + insert_before`, where
/// `static_insertion_index` counts only *static* children. But the interpreter's
/// `child(index)` indexes *live* `childNodes[index]`, so the already-live "A"
/// text shifts the live index: `child(2)` resolved to `span { "1" }` instead of
/// `span { "2" }`, and "B" was inserted before `span { "1" }`.
///
/// Expected order `span0, A, span1, B, span2`; the bug produces
/// `span0, A, B, span1, span2`. A fresh rebuild is correct (fill order
/// materializes the later slot first), so this only reproduces on the
/// incremental update — which is exactly what `assert_matches` (incremental DOM
/// vs fresh build) checks.
#[test]
fn dynamic_node_before_non_first_static_sibling_keeps_order() {
    fn app() -> Element {
        let show_b = generation() >= 1;
        rsx! {
            div {
                span { "0" }
                {"A"}
                span { "1" }
                if show_b {
                    "B"
                }
                span { "2" }
            }
        }
    }

    fn expected_without_b() -> Element {
        rsx! {
            div {
                span { "0" }
                "A"
                span { "1" }
                span { "2" }
            }
        }
    }

    fn expected_with_b() -> Element {
        rsx! {
            div {
                span { "0" }
                "A"
                span { "1" }
                "B"
                span { "2" }
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches(expected_without_b);

    // Incrementally create "B" before `span { "2" }` while "A" is already live.
    dom.mark_dirty(ScopeId::APP);
    let summary = oracle.render(&mut dom);
    oracle.assert_matches(expected_with_b);
    assert_eq!(summary.replaces, 0);
}

// Regression test for https://github.com/DioxusLabs/dioxus/issues/2815
#[test]
fn toggle_template() {
    fn app() -> Element {
        rsx!(
            Comp {
                if true {
                    "{true}"
                }
            }
        )
    }

    #[component]
    fn Comp(children: Element) -> Element {
        let show = generation() % 2 == 0;

        rsx! {
            if show {
                {children}
            }
        }
    }

    fn expected_true() -> Element {
        rsx! { "true" }
    }

    fn expected_empty() -> Element {
        rsx!({})
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches(expected_true);

    // Toggling rsx-block visibility should not use placeholder replacement.
    for step in 1..=4 {
        dom.mark_dirty(ScopeId::APP);
        let summary = oracle.render(&mut dom);
        if step % 2 == 0 {
            oracle.assert_matches(expected_true);
        } else {
            oracle.assert_matches(expected_empty);
        }
        assert_eq!(summary.replaces, 0);
    }
}
