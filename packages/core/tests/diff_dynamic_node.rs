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
