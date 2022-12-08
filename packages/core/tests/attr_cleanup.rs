//! dynamic attributes in dioxus necessitate an allocated node ID.
//!
//! This tests to ensure we clean it up

use dioxus::core::{ElementId, Mutation::*};
use dioxus::prelude::*;

#[test]
fn attrs_cycle() {
    let mut dom = VirtualDom::new(|cx| {
        let id = cx.generation();
        match cx.generation() % 2 {
            0 => cx.render(rsx! {
                div {}
            }),
            1 => cx.render(rsx! {
                div {
                    h1 { class: "{id}", id: "{id}" }
                }
            }),
            _ => unreachable!(),
        }
    });

    assert_eq!(
        dom.rebuild().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(1,) },
            AppendChildren { m: 1, id: ElementId(0) },
        ]
    );

    dom.mark_dirty(ScopeId(0));
    assert_eq!(
        dom.render_immediate().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(2,) },
            AssignId { path: &[0,], id: ElementId(3,) },
            SetAttribute { name: "class", value: "1", id: ElementId(3,), ns: None },
            SetAttribute { name: "id", value: "1", id: ElementId(3,), ns: None },
            ReplaceWith { id: ElementId(1,), m: 1 },
        ]
    );

    dom.mark_dirty(ScopeId(0));
    assert_eq!(
        dom.render_immediate().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(3) },
            ReplaceWith { id: ElementId(2), m: 1 }
        ]
    );

    dom.mark_dirty(ScopeId(0));
    assert_eq!(
        dom.render_immediate().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(2) },
            AssignId { path: &[0], id: ElementId(1) },
            SetAttribute { name: "class", value: "3", id: ElementId(1), ns: None },
            SetAttribute { name: "id", value: "3", id: ElementId(1), ns: None },
            ReplaceWith { id: ElementId(3), m: 1 }
        ]
    );

    // we take the node taken by attributes since we reused it
    dom.mark_dirty(ScopeId(0));
    assert_eq!(
        dom.render_immediate().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(1) },
            ReplaceWith { id: ElementId(2), m: 1 }
        ]
    );
}
