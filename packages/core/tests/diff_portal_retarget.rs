use std::{cell::Cell, collections::BTreeMap};

use dioxus::prelude::*;
use dioxus_core::{Portal, RenderTargetId, VirtualDom};
use dioxus_renderer_oracle::RendererOracle;

/// Regression guard for the portal retarget arm. When a `Portal`'s `target`
/// changes, the retarget path removes the old mount and creates a fresh one;
/// it must keep the portal scope's bookkeeping consistent so a dynamic sibling
/// sharing the portal's insertion position is still placed correctly.
///
/// The retarget here moves the portal body from `ROOT` to an inner target while
/// a sibling text node next to the portal placeholder is recreated across the
/// retarget. Both render-target mutation stacks must stay balanced — the canary
/// that the new body and the outer sibling were placed against a consistent
/// mount rather than a stale one.
#[test]
fn portal_retarget_keeps_clean_stacks_with_shared_sibling() {
    #[derive(Clone, PartialEq, Props)]
    struct AppProps {
        inner: RenderTargetId,
    }

    thread_local! {
        static ORDER: Cell<usize> = const { Cell::new(0) };
    }

    fn app(props: AppProps) -> Element {
        let order = ORDER.with(|o| o.get());
        // First render keeps the body in ROOT; later renders retarget it to the
        // inner target. The sibling text changes every render so it is recreated
        // at the portal's insertion position across the retarget.
        let target = if order == 0 {
            RenderTargetId::ROOT
        } else {
            props.inner
        };
        rsx! {
            div {
                Portal { target,
                    span { "portal-body" }
                }
                "sibling-{order}"
            }
        }
    }

    // Reserve the inner target id on a throwaway vdom so it is known before the
    // real App reads its props, then recreate it on the live vdom.
    let bootstrap = VirtualDom::new_with_props(app, AppProps { inner: RenderTargetId::ROOT });
    let inner = bootstrap.runtime().create_render_target();
    drop(bootstrap);

    let mut dom = VirtualDom::new_with_props(app, AppProps { inner });
    let _ = dom.runtime().create_render_target();
    let mut writer = BTreeMap::new();
    writer.insert(RenderTargetId::ROOT, RendererOracle::new());
    writer.insert(inner, RendererOracle::new());
    dom.rebuild(&mut writer);

    assert!(writer.get(&RenderTargetId::ROOT).unwrap().is_stack_clean());
    assert!(writer.get(&inner).unwrap().is_stack_clean());

    // Retarget the portal (ROOT -> inner) and recreate the sibling.
    ORDER.with(|o| o.set(1));
    dom.mark_dirty(dioxus_core::ScopeId::APP);
    dom.render_immediate(&mut writer);
    assert!(writer.get(&RenderTargetId::ROOT).unwrap().is_stack_clean());
    assert!(writer.get(&inner).unwrap().is_stack_clean());

    // Recreate the sibling again *after* the retarget so its placement scan
    // reads the portal scope's (post-retarget) mount.
    ORDER.with(|o| o.set(2));
    dom.mark_dirty(dioxus_core::ScopeId::APP);
    dom.render_immediate(&mut writer);

    writer
        .remove(&RenderTargetId::ROOT)
        .unwrap()
        .assert_stack_clean();
    writer.remove(&inner).unwrap().assert_stack_clean();
}
