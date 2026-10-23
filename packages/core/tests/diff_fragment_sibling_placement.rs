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

#[test]
fn deeply_nested_fragment_sibling_nested_child_changes_template() {
    #[component]
    fn Comp() -> Element {
        rsx! { b {} }
    }

    // Wrap the changing nested fragment in an intermediate one-element fragment so the
    // placement scan has to traverse `Fragment -> Fragment` via `dynamic_node_edge_element`,
    // and nest the inserting sibling inside a deeper fragment chain (> 2 `DiffFrame` levels)
    // so `PlacementResolver` also looks up an in-flight ancestor mount beyond immediate parent.
    fn app() -> Element {
        let odd = generation() % 2 == 1;

        let inner_changed = if odd {
            rsx! { Comp {} }
        } else {
            rsx! { p {} }
        };
        let middle_fragment = rsx! { {std::iter::once(inner_changed)} };
        let first = rsx! { {std::iter::once(middle_fragment)} };

        let deep_inserted = rsx! {
            if odd {
                span {}
            }
        };
        let second_inner = rsx! { {std::iter::once(deep_inserted)} };
        let second = rsx! { {std::iter::once(second_inner)} };

        rsx! {
            div { {[first, second].into_iter()} }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches_vdom(&dom);

    for _ in 0..4 {
        dom.mark_dirty(ScopeId::APP);
        oracle.render(&mut dom);
        oracle.assert_matches_vdom(&dom);
    }
}

#[test]
fn deeply_nested_fragment_sibling_nested_child_changes_template_with_anchors() {
    // In even generations the nested node has 0 anchors (empty fragment); in odd generations
    // it has 3 static element roots. If `dynamic_node_edge_element` walks `nodes[i]` from an
    // uncommitted ancestor VNode against `child_mounts[i]` after `child_mounts[i]` has already
    // committed the new 3-root template (or vice versa when reversing), `find_element_in_roots`
    // indexes `mount.slots` out of bounds and panics.
    fn app() -> Element {
        let odd = generation() % 2 == 1;

        let inner_changed = if odd {
            rsx! {
                p { "a" }
                b { "b" }
                i { "c" }
            }
        } else {
            rsx! {}
        };
        let middle_fragment = rsx! { {std::iter::once(inner_changed)} };
        let first = rsx! { {std::iter::once(middle_fragment)} };

        let deep_inserted = rsx! {
            if !odd {
                span {}
            }
        };
        let second_inner = rsx! { {std::iter::once(deep_inserted)} };
        let second = rsx! { {std::iter::once(second_inner)} };

        rsx! {
            div { {[first, second].into_iter()} }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches_vdom(&dom);

    for _ in 0..4 {
        dom.mark_dirty(ScopeId::APP);
        oracle.render(&mut dom);
        oracle.assert_matches_vdom(&dom);
    }
}
