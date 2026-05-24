use std::cell::Cell;

use dioxus::prelude::*;
use dioxus_core::ScopeId;
use dioxus_renderer_oracle::RendererOracle;

/// Diffing a sibling list larger than the diff's batching threshold
/// (`FRAGMENT_WORK_BATCH = 16` in `packages/core/src/diff/iterator.rs`) drives
/// every entry through the batched `component_props_update` fast path when
/// each pair is the same component with the same render_fn. The lookup then
/// queues the prop updates instead of recursing one frame per pair.
#[test]
fn batched_component_props_update_for_large_same_shape_fragment() {
    const N: usize = 20;

    thread_local! {
        static OFFSET: Cell<usize> = const { Cell::new(0) };
    }

    #[derive(Clone, Copy, PartialEq, Props)]
    struct ItemProps {
        value: usize,
    }

    #[allow(non_snake_case)]
    fn Item(props: ItemProps) -> Element {
        rsx! { span { "{props.value}" } }
    }

    fn app() -> Element {
        let offset = OFFSET.with(|o| o.get());
        rsx! {
            for i in 0..N {
                Item { value: i + offset }
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);

    // Bump the offset and re-render the App so every Item pair is the same
    // component but with different props — the batched
    // `component_props_update` fast path is the only way these >16 siblings
    // can diff.
    OFFSET.with(|o| o.set(o.get() + 1));
    dom.mark_dirty(ScopeId::APP);
    oracle.render(&mut dom);

    fn expected() -> Element {
        rsx! {
            for i in 0..N {
                span { "{i + 1}" }
            }
        }
    }
    oracle.assert_matches(expected);
}

/// `component_props_update` returns `None` whenever a pair in the large
/// fragment fails any of its fast-path preconditions — template mismatch,
/// non-component dynamic root, multiple roots, etc. — falling back to
/// per-pair `DiffFrame::diff_into`. This drives the `_ => return None`
/// branches in `single_root_component` and the early returns in
/// `component_props_update` that the same-shape test above skips past.
#[test]
fn batched_component_props_update_fallback_when_one_pair_mismatches() {
    const N: usize = 20;

    thread_local! {
        static MODE: Cell<usize> = const { Cell::new(0) };
    }

    #[derive(Clone, Copy, PartialEq, Props)]
    struct ItemProps {
        value: usize,
    }

    #[allow(non_snake_case)]
    fn Item(props: ItemProps) -> Element {
        rsx! { span { "{props.value}" } }
    }

    #[allow(non_snake_case)]
    fn OtherItem(props: ItemProps) -> Element {
        rsx! { p { "{props.value}" } }
    }

    fn app() -> Element {
        let mode = MODE.with(|o| o.get());
        rsx! {
            for i in 0..N {
                // Swap the middle entry to a different component so the
                // batched fast path bails on the first probe and reverts to
                // the per-pair diff loop.
                if mode == 1 && i == N / 2 {
                    OtherItem { value: i }
                } else {
                    Item { value: i }
                }
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);

    MODE.with(|o| o.set(1));
    dom.mark_dirty(ScopeId::APP);
    oracle.render(&mut dom);

    fn expected() -> Element {
        rsx! {
            for i in 0..N {
                if i == N / 2 {
                    p { "{i}" }
                } else {
                    span { "{i}" }
                }
            }
        }
    }
    oracle.assert_matches(expected);
}

/// `component_props_update`'s `single_root_component(old)?` short-circuit
/// fires when an entry isn't a component at all (e.g. a raw element with a
/// dynamic-text root). This exercises the `_ => None` arm in
/// `single_root_component` for the non-`Component` dynamic root.
#[test]
fn batched_component_props_update_fallback_for_non_component_dynamic_root() {
    const N: usize = 20;

    thread_local! {
        static OFFSET: Cell<usize> = const { Cell::new(0) };
    }

    fn app() -> Element {
        let offset = OFFSET.with(|o| o.get());
        // Top-level `{ ... }` expressions render to a vnode whose template
        // root is a `DynamicNode::Text`, so each sibling here has a single
        // dynamic — but non-component — root. The batched fast path probes
        // it with `single_root_component`, hits the non-Component arm, and
        // falls through to the per-pair diff loop.
        rsx! {
            for i in 0..N {
                "item-{i + offset}"
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);

    OFFSET.with(|o| o.set(o.get() + 1));
    dom.mark_dirty(ScopeId::APP);
    oracle.render(&mut dom);

    fn expected() -> Element {
        rsx! {
            for i in 0..N {
                "item-{i + 1}"
            }
        }
    }
    oracle.assert_matches(expected);
}
