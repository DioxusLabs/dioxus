//! Exercise the `(false, true, Some(_))` arm of `diff_dynamic_attribute`
//! (packages/core/src/diff/attributes.rs:196), where the same dynamic
//! attribute key transitions from a value to a listener across renders.
//!
//! The fuzz harness's `dynamic_attr_name` couples a value byte's high bit to
//! both the attribute-name format and listener-ness, so the byte stream of
//! fuzz inputs can never produce a value attribute and a listener that
//! share a key. The only way to reach that arm is to hand-construct the
//! attribute lists.

use dioxus::prelude::*;
use dioxus_core::{AttributeValue, ScopeId, generation};
use dioxus_renderer_oracle::{RendererOracle, SnapshotNode};

/// The first element with the given tag in the snapshot.
fn find_element<'a>(nodes: &'a [SnapshotNode], tag: &str) -> Option<&'a SnapshotNode> {
    for node in nodes {
        if let SnapshotNode::Element { tag: node_tag, children, .. } = node {
            if node_tag == tag {
                return Some(node);
            }
            if let Some(found) = find_element(children, tag) {
                return Some(found);
            }
        }
    }
    None
}

#[test]
fn value_to_listener_at_same_key_clears_old_value() {
    fn app() -> Element {
        match generation() {
            0 => {
                let attrs = vec![Attribute::new("onclick", "raw", None, false)];
                rsx! { button { ..attrs } }
            }
            _ => {
                let listeners = vec![Attribute::new(
                    "onclick",
                    AttributeValue::listener(|_: Event<()>| {}),
                    None,
                    false,
                )];
                rsx! { button { ..listeners } }
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);

    dom.mark_dirty(ScopeId::APP);
    let summary = oracle.render(&mut dom);
    // The transition installs a listener and clears the old "onclick" value
    // attribute, so the diff emits one `set_attribute` (to AttributeValue::None
    // on line 196) followed by the listener install.
    assert!(
        summary.set_attrs >= 1,
        "expected at least one set_attribute call, got summary={summary:?}",
    );

    // The end state is what matters: the button must carry a `click` listener
    // and must no longer expose the stale `onclick` value attribute.
    let snapshot = oracle.snapshot();
    let SnapshotNode::Element { attrs, listeners, .. } =
        find_element(&snapshot, "button").expect("button should exist")
    else {
        unreachable!("find_element only returns elements")
    };
    assert!(
        listeners.iter().any(|name| name == "click"),
        "expected a `click` listener to be installed, listeners={listeners:?}",
    );
    assert!(
        !attrs.iter().any(|a| a.name == "onclick"),
        "stale `onclick` value attribute should be cleared, attrs={attrs:?}",
    );
}
