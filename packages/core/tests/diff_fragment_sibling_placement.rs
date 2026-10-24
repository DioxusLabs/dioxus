//! Placement scans of fragment siblings during a fragment diff.
//!
//! Stable fragment children are diffed in place, in order. When a later child places a new
//! node and has no live edge of its own, placement walks up to the fragment and scans the
//! child's siblings. The siblings already diffed hold the new view in their mounts (and in
//! their nested fragment child mounts), so scanning them through the parent's old child
//! vnodes paired an old template with a new mount and indexed slots that do not exist.
use dioxus::prelude::*;
use dioxus_core::{ScopeId, generation};
use dioxus_renderer_oracle::RendererOracle;

#[test]
fn earlier_sibling_nested_child_changes_template() {
    #[component]
    fn Comp() -> Element {
        rsx! { b {} }
    }

    // The tree is `div { [first, second] }`, a fragment of two children whose templates
    // never change, so both are diffed in place. Every render flips between:
    //
    //   even generations: first = [ p {} ],    second = (empty)
    //   odd generations:  first = [ Comp {} ], second = span {}
    fn app() -> Element {
        let odd = generation() % 2 == 1;

        // A one-node fragment whose node changes template: diffing it gives the nested
        // node a new mount.
        let nested = if odd {
            rsx! { Comp {} }
        } else {
            rsx! { p {} }
        };
        let first = rsx! { {std::iter::once(nested)} };

        // No static element of its own, so inserting `span` scans back to `first`, which
        // has already been diffed.
        let second = rsx! {
            if odd {
                span {}
            }
        };

        rsx! {
            div { {[first, second].into_iter()} }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches_vdom(&dom);

    // Flip both ways twice; each render must match a fresh build of the same tree.
    for _ in 0..4 {
        dom.mark_dirty(ScopeId::APP);
        oracle.render(&mut dom);
        oracle.assert_matches_vdom(&dom);
    }
}
