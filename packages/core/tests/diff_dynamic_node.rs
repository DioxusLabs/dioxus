use dioxus::dioxus_core::{ElementId, Mutation::*};
use dioxus::prelude::*;
use pretty_assertions::assert_eq;

#[test]
fn toggle_option_text() {
    let mut dom = VirtualDom::new(|| {
        let gen = generation();
        let text = if gen % 2 != 0 { Some("hello") } else { None };

        rsx! {
            div {
                {text}
            }
        }
    });

    // load the div and then assign the None as a placeholder
    assert_eq!(
        dom.rebuild_to_vec().edits,
        [
            LoadTemplate { index: 0, id: ElementId(1,) },
            AssignId { path: &[0], id: ElementId(2,) },
            AppendChildren { id: ElementId(0), m: 1 },
        ]
    );

    // Rendering again should replace the placeholder with an text node
    dom.mark_dirty(ScopeId::APP);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [
            CreateTextNode { value: "hello".to_string(), id: ElementId(3,) },
            ReplaceWith { id: ElementId(2,), m: 1 },
        ]
    );

    // Rendering again should replace the placeholder with an text node
    dom.mark_dirty(ScopeId::APP);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [
            CreatePlaceholder { id: ElementId(2,) },
            ReplaceWith { id: ElementId(3,), m: 1 },
        ]
    );
}
