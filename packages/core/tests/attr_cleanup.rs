//! dynamic attributes in dioxus necessitate an allocated node ID.
//!
//! This tests to ensure we clean it up

use dioxus::dioxus_core::{ElementId, Mutation::*};
use dioxus::prelude::*;

#[test]
fn attrs_cycle() {
    let mut dom = VirtualDom::new(|| {
        let id = generation();
        match id % 2 {
            0 => rsx! { div {} },
            1 => rsx! {
                div { h1 { class: "{id}", id: "{id}" } }
            },
            _ => unreachable!(),
        }
    });

    assert_eq!(
        dom.rebuild_to_vec().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(1,) },
            AppendChildren { m: 1, id: ElementId(0) },
        ]
    );

    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(2,) },
            AssignId { path: &[0,], id: ElementId(3,) },
            SetAttribute { name: "class", value: "1".into_value(), id: ElementId(3,), ns: None },
            SetAttribute { name: "id", value: "1".into_value(), id: ElementId(3,), ns: None },
            ReplaceWith { id: ElementId(1,), m: 1 },
        ]
    );

    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(1) },
            ReplaceWith { id: ElementId(2), m: 1 }
        ]
    );

    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(2) },
            AssignId { path: &[0], id: ElementId(3) },
            SetAttribute {
                name: "class",
                value: dioxus_core::AttributeValue::Text("3".to_string()),
                id: ElementId(3),
                ns: None
            },
            SetAttribute {
                name: "id",
                value: dioxus_core::AttributeValue::Text("3".to_string()),
                id: ElementId(3),
                ns: None
            },
            ReplaceWith { id: ElementId(1), m: 1 }
        ]
    );

    // we take the node taken by attributes since we reused it
    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(1) },
            ReplaceWith { id: ElementId(2), m: 1 }
        ]
    );
}
