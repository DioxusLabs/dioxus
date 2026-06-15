use dioxus::prelude::*;
use dioxus_core::{AttributeValue, Mutation, ScopeId, VirtualDom, generation};

#[test]
fn unchanged_dynamic_attrs_emit_no_scope() {
    fn app() -> Element {
        let attrs = vec![
            Attribute::new("class", "stable", None, false),
            Attribute::new("id", "stable", None, false),
        ];

        rsx! {
            div { ..attrs }
        }
    }

    let mut dom = VirtualDom::new(app);
    _ = dom.rebuild_to_vec();

    dom.mark_dirty(ScopeId::APP);
    let mutations = dom.render_immediate_to_vec();

    assert!(
        mutations.edits.is_empty(),
        "unchanged dynamic attributes should not emit mutations: {:?}",
        mutations.edits
    );
}

#[test]
fn changed_dynamic_attrs_share_one_scope() {
    fn app() -> Element {
        let g = generation();
        let attrs = vec![
            Attribute::new(
                "class",
                AttributeValue::Text(format!("class-{g}")),
                None,
                false,
            ),
            Attribute::new("id", AttributeValue::Text(format!("id-{g}")), None, false),
        ];

        rsx! {
            div { ..attrs }
        }
    }

    let mut dom = VirtualDom::new(app);
    _ = dom.rebuild_to_vec();

    dom.mark_dirty(ScopeId::APP);
    let mutations = dom.render_immediate_to_vec();
    let edits = &mutations.edits;

    assert_eq!(push_count(edits), 1, "{edits:?}");
    assert_eq!(pop_count(edits), 1, "{edits:?}");
    assert_eq!(edits.len(), 4, "{edits:?}");
    assert!(matches!(edits[0], Mutation::PushId { .. }), "{edits:?}");
    assert_attr_text(&edits[1], "class", "class-1");
    assert_attr_text(&edits[2], "id", "id-1");
    assert!(matches!(edits[3], Mutation::Pop), "{edits:?}");
}

#[test]
fn value_to_listener_swap_shares_one_scope() {
    fn app() -> Element {
        let attrs = match generation() {
            0 => vec![Attribute::new("onclick", "raw", None, false)],
            _ => vec![Attribute::new(
                "onclick",
                AttributeValue::listener(|_: Event<()>| {}),
                None,
                false,
            )],
        };

        rsx! {
            button { ..attrs }
        }
    }

    let mut dom = VirtualDom::new(app);
    _ = dom.rebuild_to_vec();

    dom.mark_dirty(ScopeId::APP);
    let mutations = dom.render_immediate_to_vec();
    let edits = &mutations.edits;

    assert_eq!(push_count(edits), 1, "{edits:?}");
    assert_eq!(pop_count(edits), 1, "{edits:?}");
    assert_eq!(edits.len(), 4, "{edits:?}");
    assert!(matches!(edits[0], Mutation::PushId { .. }), "{edits:?}");
    assert_attr_none(&edits[1], "onclick");
    assert!(
        matches!(&edits[2], Mutation::NewEventListener { name } if name == "click"),
        "{edits:?}"
    );
    assert!(matches!(edits[3], Mutation::Pop), "{edits:?}");
}

#[test]
fn empty_root_rebuild_emits_no_empty_append_scope() {
    fn app() -> Element {
        rsx! {}
    }

    let mut dom = VirtualDom::new(app);
    let mutations = dom.rebuild_to_vec();

    assert!(
        mutations.edits.is_empty(),
        "empty root rebuild should not emit an empty append scope: {:?}",
        mutations.edits
    );
}

fn push_count(edits: &[Mutation]) -> usize {
    edits
        .iter()
        .filter(|edit| matches!(edit, Mutation::PushId { .. }))
        .count()
}

fn pop_count(edits: &[Mutation]) -> usize {
    edits
        .iter()
        .filter(|edit| matches!(edit, Mutation::Pop))
        .count()
}

fn assert_attr_text(edit: &Mutation, expected_name: &str, expected_value: &str) {
    assert!(
        matches!(
            edit,
            Mutation::SetAttribute {
                name,
                ns: None,
                value: AttributeValue::Text(value),
            } if name == expected_name && value == expected_value
        ),
        "{edit:?}"
    );
}

fn assert_attr_none(edit: &Mutation, expected_name: &str) {
    assert!(
        matches!(
            edit,
            Mutation::SetAttribute {
                name,
                ns: None,
                value: AttributeValue::None,
            } if name == expected_name
        ),
        "{edit:?}"
    );
}
