use std::cell::Cell;

use dioxus::prelude::*;
use dioxus_core::{Portal, RenderTargetId, VirtualDom};
use dioxus_renderer_oracle::RendererOracle;

/// Reordering a keyed list whose entries are `Portal`s exercises the
/// cross-render-target paths in `push_all_root_nodes`: the keyed-middle
/// splice walks each new sibling in the *outer* render target, but the
/// portal's body lives in a different target, so the dynamic-text /
/// static-root arms have to early-return `0` instead of pushing into the
/// wrong target.
#[test]
fn keyed_portal_list_reorder_does_not_push_into_wrong_target() {
    #[derive(Clone, PartialEq, Props)]
    struct AppProps {
        target: RenderTargetId,
    }

    thread_local! {
        static ORDER: Cell<usize> = const { Cell::new(0) };
    }

    fn app(props: AppProps) -> Element {
        let order = ORDER.with(|o| o.get());
        let keys: [&'static str; 5] = if order == 0 {
            ["a", "b", "c", "d", "e"]
        } else {
            // Shuffle so the keyed-middle splice actually has to move
            // entries past each other — and not just trim from the ends.
            ["c", "a", "e", "b", "d"]
        };
        rsx! {
            div {
                for (i, key) in keys.iter().enumerate() {
                    Portal {
                        key: "{key}",
                        target: props.target,
                        // Alternate dynamic-text and static-element portal
                        // bodies so the cross-target arms of
                        // `push_all_root_nodes` exercise both the
                        // `Some((_, Text(_)))` path and the static `None`
                        // path during the keyed-middle splice.
                        if i % 2 == 0 {
                            "portal-{key}"
                        } else {
                            span { "portal-{key}" }
                        }
                    }
                }
            }
        }
    }

    // Spin up the vdom and reserve a second render target before the App
    // gets a chance to read its props.
    let mut bootstrap = VirtualDom::new_with_props(app, AppProps { target: RenderTargetId::ROOT });
    let inner_target = bootstrap.runtime().create_render_target();
    drop(bootstrap);

    let mut dom = VirtualDom::new_with_props(app, AppProps { target: inner_target });
    let _ = dom.runtime().create_render_target();
    dom.insert_render_target(RenderTargetId::ROOT, RendererOracle::new());
    dom.insert_render_target(inner_target, RendererOracle::new());
    dom.rebuild();
    let mut outer = dom
        .take_render_target::<RendererOracle>(RenderTargetId::ROOT)
        .unwrap();
    let mut inner = dom
        .take_render_target::<RendererOracle>(inner_target)
        .unwrap();

    // Sanity: outer holds the portal placeholders, inner holds the bodies.
    assert!(outer.is_stack_clean());
    assert!(inner.is_stack_clean());

    ORDER.with(|o| o.set(1));
    dom.mark_dirty(dioxus_core::ScopeId::APP);
    dom.insert_render_target(RenderTargetId::ROOT, outer);
    dom.insert_render_target(RenderTargetId(1), inner);
    dom.render_immediate();
    outer = dom.take_render_target(RenderTargetId::ROOT).unwrap();
    inner = dom.take_render_target(RenderTargetId(1)).unwrap();

    // Stack still clean after the reorder, which is the canary for the
    // cross-target push paths going wrong (an unbalanced push leaves the
    // mutation stack with extra entries).
    outer.assert_stack_clean();
    inner.assert_stack_clean();
}
