use dioxus::prelude::*;
use dioxus_core::{ScopeId, generation};
use dioxus_renderer_oracle::RendererOracle;

/// As we clean up old templates, the ID for the node should cycle
#[test]
fn cycling_elements() {
    fn app() -> Element {
        match generation() % 2 {
            0 => rsx! { div { "wasd" } },
            _ => rsx! { div { "abcd" } },
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);

    for _ in 1..=3 {
        dom.mark_dirty(ScopeId::APP);
        let summary = oracle.render(&mut dom);
        // Load the new template, then remove the old one.
        assert_eq!(summary.loads, 1);
        assert_eq!(summary.removes, 1);
    }
}
