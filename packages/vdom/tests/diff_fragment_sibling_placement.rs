//! Placement scans of fragment siblings during a fragment diff.
//!
//! Stable fragment children are diffed in place, in order. When a later child places a new
//! node and has no live edge of its own, placement walks up to the fragment and scans the
//! child's siblings. The siblings already diffed hold the new view in their mounts (and in
//! their nested fragment child mounts), so scanning them through the parent's old child
//! vnodes paired an old template with a new mount and indexed slots that do not exist.
use dioxus::prelude::*;
use dioxus_renderer_oracle::RendererOracle;
use dioxus_vdom::{ScopeId, generation};

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
fn deeply_nested_insert_after_earlier_sibling_grows() {
    // The tree is `i {} {first} {second}`, where `second` wraps an inserting node in two
    // vnodes whose templates never change. Every render flips between:
    //
    //   even generations: first = (empty), second = (empty)
    //   odd generations:  first = p {},    second = span {}
    fn app() -> Element {
        let odd = generation() % 2 == 1;

        let inserted = rsx! {
            if odd {
                span {}
            }
        };
        let second = rsx! { {std::iter::once(inserted)} };

        // `first` is diffed before `second`. Inserting `span` walks up two vnodes to the
        // root, which is still mid-diff, so `first` has to be read in its new shape (`p`)
        // rather than the committed empty one, or `span` lands right after `i`.
        rsx! {
            i {}
            {odd.then(|| rsx! { p {} }).into_iter()}
            {std::iter::once(second)}
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
fn insert_in_suspense_boundary_after_earlier_sibling_changes_template() {
    #[component]
    fn Comp() -> Element {
        rsx! { b {} }
    }

    // The tree is `div { [first] SuspenseBoundary { [second] } }`. Every render flips between:
    //
    //   even generations: first = p {},    second = (empty)
    //   odd generations:  first = Comp {}, second = span {}
    fn app() -> Element {
        let odd = generation() % 2 == 1;

        let first = if odd {
            rsx! { Comp {} }
        } else {
            rsx! { p {} }
        };

        let second = rsx! {
            if odd {
                span {}
            }
        };

        // The suspense boundary diffs its children without the outer diff frames, so
        // inserting `span` scans `first` through the root's committed (old) vnode while
        // `first`'s mount already holds the new template.
        rsx! {
            div {
                {std::iter::once(first)}
                SuspenseBoundary { fallback: |_| rsx! {}, {std::iter::once(second)} }
            }
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
fn insert_in_deep_fragment_after_sibling_changes_root_count() {
    // The same route as the test above, with no suspense boundary: `i {} {first} {second}`.
    // `first` and `second` are adjacent dynamic nodes, so they share one anchor and one
    // insertion position. Every render flips between:
    //
    //   even generations: first = a {} b {} i {}, second = (empty)
    //   odd generations:  first = p {},            second = span {}
    //
    // `first` is diffed before `second`, so when `span` is placed, `first`'s mount already
    // holds `p` (one root, one slot) while the root's committed vnode still describes
    // `a {} b {} i {}` (three roots). Scanning the previous sibling through that vnode then
    // indexes an anchor `first`'s mount no longer has, and panics in `Mount::anchor_slot`
    // (the `mount.rs:242` report on 0.8.0-alpha.3).
    fn app() -> Element {
        let odd = generation() % 2 == 1;

        let first = if odd {
            rsx! { p {} }
        } else {
            rsx! {
                a {}
                b {}
                i {}
            }
        };

        let inserted = rsx! {
            if odd {
                span {}
            }
        };
        let second = rsx! { {std::iter::once(inserted)} };

        rsx! {
            i {}
            {first}
            {second}
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
