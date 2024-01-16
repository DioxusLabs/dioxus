use dioxus::dioxus_core::{ElementId, Mutation::*};
use dioxus::prelude::*;

/// As we clean up old templates, the ID for the node should cycle
#[test]
fn cycling_elements() {
    let mut dom = VirtualDom::new(|| match generation() % 2 {
        0 => rsx! { div { "wasd" } },
        1 => rsx! { div { "abcd" } },
        _ => unreachable!(),
    });

    {
        let edits = dom.rebuild_to_vec().santize();
        assert_eq!(
            edits.edits,
            [
                LoadTemplate { name: "template", index: 0, id: ElementId(1,) },
                AppendChildren { m: 1, id: ElementId(0) },
            ]
        );
    }

    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(2,) },
            ReplaceWith { id: ElementId(1,), m: 1 },
        ]
    );

    // notice that the IDs cycle back to ElementId(1), preserving a minimal memory footprint
    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(1,) },
            ReplaceWith { id: ElementId(2,), m: 1 },
        ]
    );

    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(2,) },
            ReplaceWith { id: ElementId(1,), m: 1 },
        ]
    );
}
