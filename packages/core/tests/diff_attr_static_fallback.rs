//! Exercise the case where a disappearing dynamic attribute was shadowing a
//! static template attribute at the same `(name, namespace)` key. After the
//! dynamic attribute disappears, the diff must restore the static value.

use dioxus::prelude::*;
use dioxus_core::{Attribute, ScopeId, generation};
use dioxus_renderer_oracle::{RendererOracle, SnapshotNode};

/// Find the value of `attr` on the first `tag` element anywhere in the snapshot.
fn attr_value(nodes: &[SnapshotNode], tag: &str, attr: &str) -> Option<String> {
    for node in nodes {
        if let SnapshotNode::Element { tag: node_tag, attrs, children, .. } = node {
            if node_tag == tag {
                if let Some(found) = attrs.iter().find(|a| a.name == attr) {
                    return Some(found.value.clone());
                }
            }
            if let Some(found) = attr_value(children, tag, attr) {
                return Some(found);
            }
        }
    }
    None
}

#[test]
fn static_attribute_resurfaces_when_dynamic_disappears() {
    fn app() -> Element {
        // The template carries a *static* `class="from-template"` attribute.
        // On the first generation we layer a *dynamic* `class="overlay"` on
        // top of it via `..attrs`; on the next generation the dynamic
        // attribute disappears, which must restore the static value.
        let attrs: Vec<Attribute> = if generation() == 0 {
            vec![Attribute::new("class", "overlay", None, false)]
        } else {
            Vec::new()
        };

        rsx! {
            div {
                class: "from-template",
                ..attrs,
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);

    dom.mark_dirty(ScopeId::APP);
    let summary = oracle.render(&mut dom);

    // The dynamic attribute disappears, so the diff must call
    // `remove_attribute_or_write_fallback`, find the static template attr,
    // and emit a `set_attribute` restoring its value. Anything ≥ 1 means
    // the fallback Some(value) branch fired.
    assert!(
        summary.set_attrs >= 1,
        "expected static template attribute to be restored, got summary={summary:?}",
    );
    // ...and it must restore the *static* value, not leave the stale dynamic
    // value or an empty attribute behind.
    assert_eq!(
        attr_value(&oracle.snapshot(), "div", "class").as_deref(),
        Some("from-template"),
        "static `class` value should be restored, snapshot={:#?}",
        oracle.snapshot(),
    );
}

#[test]
fn nested_static_attribute_resurfaces_when_dynamic_disappears() {
    // Same scenario as above but on a deeper element path, so
    // `template_node_at_path` recurses through `element_child(...)`
    // (attributes.rs:291) before resolving the owning element.
    fn app() -> Element {
        let attrs: Vec<Attribute> = if generation() == 0 {
            vec![Attribute::new("id", "overlay", None, false)]
        } else {
            Vec::new()
        };

        rsx! {
            section {
                div {
                    span {
                        id: "deep-static",
                        ..attrs,
                    }
                }
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);

    dom.mark_dirty(ScopeId::APP);
    let summary = oracle.render(&mut dom);

    assert!(
        summary.set_attrs >= 1,
        "expected deep static attribute to be restored, got summary={summary:?}",
    );
    assert_eq!(
        attr_value(&oracle.snapshot(), "span", "id").as_deref(),
        Some("deep-static"),
        "static `id` value should be restored on the nested span, snapshot={:#?}",
        oracle.snapshot(),
    );
}
