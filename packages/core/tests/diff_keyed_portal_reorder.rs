use std::{cell::Cell, collections::BTreeMap};

use dioxus::prelude::*;
use dioxus_core::{Portal, RenderTargetId, VirtualDom};
use dioxus_renderer_oracle::{RendererOracle, SnapshotNode};

/// All text content in a render target's snapshot, in document order.
fn all_text(nodes: &[SnapshotNode]) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(nodes: &[SnapshotNode], out: &mut Vec<String>) {
        for node in nodes {
            match node {
                SnapshotNode::Text(text) => out.push(text.clone()),
                SnapshotNode::Element { children, .. } => walk(children, out),
            }
        }
    }
    walk(nodes, &mut out);
    out
}

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
            // entries past each other - and not just trim from the ends.
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
    let bootstrap = VirtualDom::new_with_props(app, AppProps { target: RenderTargetId::ROOT });
    let inner_target = bootstrap.runtime().create_render_target();
    drop(bootstrap);

    let mut dom = VirtualDom::new_with_props(app, AppProps { target: inner_target });
    let _ = dom.runtime().create_render_target();
    let mut writer = BTreeMap::new();
    writer.insert(RenderTargetId::ROOT, RendererOracle::new());
    writer.insert(inner_target, RendererOracle::new());
    dom.rebuild(&mut writer);

    // Sanity: outer holds the portal placeholders, inner holds the bodies.
    assert!(writer.get(&RenderTargetId::ROOT).unwrap().is_stack_clean());
    assert!(writer.get(&inner_target).unwrap().is_stack_clean());

    ORDER.with(|o| o.set(1));
    dom.mark_dirty(dioxus_core::ScopeId::APP);
    dom.render_immediate(&mut writer);

    // All five reordered bodies must end up in the inner target, and none may
    // leak into ROOT (which holds only the placeholders). Stack balance alone
    // would not catch a body pushed into the wrong - but still valid - target.
    let inner_text = all_text(&writer.get(&inner_target).unwrap().snapshot());
    for key in ["a", "b", "c", "d", "e"] {
        let body = format!("portal-{key}");
        assert!(
            inner_text.contains(&body),
            "inner target is missing `{body}`, inner text={inner_text:?}",
        );
    }
    let root_text = all_text(&writer.get(&RenderTargetId::ROOT).unwrap().snapshot());
    assert!(
        root_text.iter().all(|t| !t.starts_with("portal-")),
        "portal bodies must not leak into ROOT, root text={root_text:?}",
    );

    // Stack still clean after the reorder, which is the canary for the
    // cross-target push paths going wrong (an unbalanced push leaves the
    // mutation stack with extra entries).
    writer
        .remove(&RenderTargetId::ROOT)
        .unwrap()
        .assert_stack_clean();
    writer.remove(&inner_target).unwrap().assert_stack_clean();
}
