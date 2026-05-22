use super::*;
use crate::vdom_snapshot::{assert_no_mutations, fresh_snapshot, vdom_snapshot};
use dioxus::prelude::*;
use dioxus_core::{Attribute, AttributeValue, Event, ScopeId, VirtualDom, generation};

fn simple_app() -> Element {
    rsx! {
        main { class: "root", "hello" }
    }
}

fn listener_app() -> Element {
    rsx! {
        button { onclick: move |_| {}, "go" }
    }
}

fn simple_app_with_different_attr() -> Element {
    rsx! {
        main { class: "different", "hello" }
    }
}

fn empty_dynamic_slot_app() -> Element {
    let show = false;
    rsx! {
        main {
            if show {
                span { "hidden" }
            }
        }
    }
}

fn render_app(app: fn() -> Element) -> RendererOracle {
    let mut vdom = VirtualDom::new(app);
    let mut renderer = RendererOracle::new();
    renderer.rebuild(&mut vdom);
    renderer
}

#[test]
fn rebuilds_static_tree() {
    let snapshot = fresh_snapshot(simple_app);
    assert_eq!(
        snapshot,
        vec![SnapshotNode::Element {
            tag: "main".to_string(),
            namespace: None,
            attrs: vec![SnapshotAttr {
                name: "class".to_string(),
                namespace: None,
                value: "root".to_string(),
            }],
            listeners: Vec::new(),
            children: vec![SnapshotNode::Text("hello".to_string())],
        }]
    );
}

#[test]
fn tracks_event_listeners() {
    let snapshot = fresh_snapshot(listener_app);
    match &snapshot[..] {
        [SnapshotNode::Element { listeners, .. }] => assert_eq!(listeners, &["click"]),
        other => panic!("unexpected snapshot: {other:#?}"),
    }
}

#[test]
fn vdom_snapshot_removes_listener_shadowed_by_later_none_attr() {
    fn app() -> Element {
        let attrs = vec![Attribute::new("onclick", AttributeValue::None, None, false)];

        rsx! {
            button {
                onclick: move |_| {},
                ..attrs,
            }
        }
    }

    let mut vdom = VirtualDom::new(app);
    vdom.rebuild_in_place();
    match &vdom_snapshot(&vdom)[..] {
        [
            SnapshotNode::Element {
                attrs, listeners, ..
            },
        ] => {
            assert!(attrs.is_empty());
            assert!(listeners.is_empty());
        }
        other => panic!("unexpected snapshot: {other:#?}"),
    }
}

#[test]
#[should_panic(expected = "renderer DOM diverged from expected rsx tree")]
fn assert_matches_rejects_stale_listener_shadowed_by_attr() {
    fn expected() -> Element {
        let attrs = vec![Attribute::new("onclick", AttributeValue::None, None, false)];

        rsx! {
            button {
                onclick: move |_| {},
                ..attrs,
                "go"
            }
        }
    }

    render_app(listener_app).assert_matches(expected);
}

#[test]
fn vdom_snapshot_removes_attr_shadowed_by_later_listener() {
    fn app() -> Element {
        let attrs = vec![Attribute::new("onclick", "raw-listener", None, false)];
        let listeners = vec![Attribute::new(
            "onclick",
            AttributeValue::listener(|_: Event<()>| {}),
            None,
            false,
        )];

        rsx! {
            button {
                ..attrs,
                ..listeners,
            }
        }
    }

    let mut vdom = VirtualDom::new(app);
    vdom.rebuild_in_place();
    match &vdom_snapshot(&vdom)[..] {
        [
            SnapshotNode::Element {
                attrs, listeners, ..
            },
        ] => {
            assert!(attrs.is_empty());
            assert_eq!(listeners, &["click"]);
        }
        other => panic!("unexpected snapshot: {other:#?}"),
    }
}

#[test]
fn empty_dynamic_slots_are_not_snapshot_nodes() {
    let snapshot = fresh_snapshot(empty_dynamic_slot_app);
    assert_eq!(
        snapshot,
        vec![SnapshotNode::Element {
            tag: "main".to_string(),
            namespace: None,
            attrs: Vec::new(),
            listeners: Vec::new(),
            children: Vec::new(),
        }]
    );
}

#[test]
fn asserts_no_mutations_for_idle_vdom() {
    let mut vdom = VirtualDom::new(simple_app);
    let mut renderer = RendererOracle::new();
    vdom.rebuild(&mut renderer);
    renderer.assert_stack_clean();
    assert_no_mutations(&mut vdom);
}

#[test]
fn assert_matches_happy_path() {
    let mut vdom = VirtualDom::new(simple_app);
    let mut renderer = RendererOracle::new();
    renderer.rebuild(&mut vdom);
    renderer.assert_matches(simple_app);
}

#[test]
fn assert_matches_round_trips_listeners() {
    let mut vdom = VirtualDom::new(listener_app);
    let mut renderer = RendererOracle::new();
    renderer.rebuild(&mut vdom);
    renderer.assert_matches(listener_app);
}

#[test]
fn snapshot_eq_matches_equal_visible_trees_without_allocated_snapshots() {
    let left = render_app(simple_app);
    let right = render_app(simple_app);
    assert!(left.snapshot_eq(&right));
}

#[test]
fn snapshot_eq_detects_visible_tree_differences() {
    let left = render_app(simple_app);
    let right = render_app(simple_app_with_different_attr);
    assert!(!left.snapshot_eq(&right));
}

#[test]
fn snapshot_eq_ignores_empty_dynamic_placeholders() {
    let left = render_app(empty_dynamic_slot_app);
    let right = render_app(empty_dynamic_slot_app);
    assert!(left.snapshot_eq(&right));
}

#[test]
fn renderer_walks_states_in_order() {
    fn app() -> Element {
        match generation() {
            0 => rsx! { div { "a" } },
            1 => rsx! { div { "b" } },
            _ => rsx! { div { "c" } },
        }
    }

    fn expected_a() -> Element {
        rsx! { div { "a" } }
    }

    fn expected_b() -> Element {
        rsx! { div { "b" } }
    }

    fn expected_c() -> Element {
        rsx! { div { "c" } }
    }

    let mut vdom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut vdom);
    oracle.assert_matches(expected_a);

    vdom.mark_dirty(ScopeId::APP);
    oracle.render(&mut vdom);
    oracle.assert_matches(expected_b);

    vdom.mark_dirty(ScopeId::APP);
    oracle.render(&mut vdom);
    oracle.assert_matches(expected_c);
}

#[test]
fn renderer_tracks_identity_for_moved_nodes() {
    fn app() -> Element {
        let keys: &[i32] = match generation() {
            0 => &[0, 1, 2, 3],
            1 => &[3, 0, 1, 2],
            _ => &[2, 3, 0, 1],
        };

        rsx! {
            for k in keys {
                div { key: "{k}", id: "{k}", "{k}" }
            }
        }
    }

    let mut vdom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut vdom);
    let first = oracle.identities_by_attr("id");

    vdom.mark_dirty(ScopeId::APP);
    oracle.render(&mut vdom);
    assert_identities_preserved(&first, &oracle.identities_by_attr("id"), "id", 1);
    let second = oracle.identities_by_attr("id");

    vdom.mark_dirty(ScopeId::APP);
    oracle.render(&mut vdom);
    assert_identities_preserved(&second, &oracle.identities_by_attr("id"), "id", 2);
}

#[test]
fn renderer_can_run_assertions_between_steps() {
    use std::cell::Cell;

    fn app() -> Element {
        match generation() {
            0 => rsx! { div { "a" } },
            1 => rsx! { div { "b" } },
            _ => rsx! { div { "c" } },
        }
    }

    let calls = Cell::new(0);
    let mut vdom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut vdom);

    calls.set(calls.get() + 1);
    vdom.mark_dirty(ScopeId::APP);
    oracle.render(&mut vdom);

    calls.set(calls.get() + 1);
    vdom.mark_dirty(ScopeId::APP);
    oracle.render(&mut vdom);

    assert_eq!(calls.get(), 2);
}

#[test]
#[should_panic(expected = "node identity for `id=hot` was not preserved")]
fn identity_check_catches_recreation() {
    // Two unkeyed elements of different tag — the diff has to drop the old
    // node and create a new one. The identity comparison catches that.
    fn app() -> Element {
        match generation() {
            0 => rsx! { div { id: "hot", "before" } },
            _ => rsx! { span { id: "hot", "after" } },
        }
    }

    let mut vdom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut vdom);
    let previous = oracle.identities_by_attr("id");

    vdom.mark_dirty(ScopeId::APP);
    oracle.render(&mut vdom);
    assert_identities_preserved(&previous, &oracle.identities_by_attr("id"), "id", 1);
}

#[test]
fn edit_summary_counts_rebuild_then_in_place_patch() {
    // First step builds the tree; rerender with the same shape but a
    // different *dynamic* text body should patch in place — same template,
    // just a new value for the dynamic slot.
    fn app() -> Element {
        let value = match generation() {
            0 => "alpha",
            _ => "beta",
        };
        rsx! { div { id: "0", "{value}" } }
    }

    fn expected_alpha() -> Element {
        rsx! { div { id: "0", "alpha" } }
    }

    fn expected_beta() -> Element {
        rsx! { div { id: "0", "beta" } }
    }

    let mut vdom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();

    let rebuild = oracle.rebuild(&mut vdom);
    oracle.assert_matches(expected_alpha);
    assert!(
        rebuild.loads >= 1,
        "rebuild should load at least one template"
    );

    vdom.mark_dirty(ScopeId::APP);
    let patch = oracle.render(&mut vdom);
    oracle.assert_matches(expected_beta);
    assert_eq!(
        patch.loads, 0,
        "in-place text patch should not load templates"
    );
    assert_eq!(patch.set_texts, 1, "exactly one text patch expected");
    assert_eq!(patch.removes, 0);
    assert_eq!(patch.replaces, 0);
}

#[test]
#[should_panic(expected = "renderer DOM diverged from expected rsx tree")]
fn assert_matches_fails_on_divergence() {
    fn other() -> Element {
        rsx! { main { class: "different", "hello" } }
    }
    let mut vdom = VirtualDom::new(simple_app);
    let mut renderer = RendererOracle::new();
    renderer.rebuild(&mut vdom);
    renderer.assert_matches(other);
}

fn assert_identities_preserved(
    previous: &[(String, OracleNodeId)],
    current: &[(String, OracleNodeId)],
    attr: &str,
    step: usize,
) {
    for (value, previous_id) in previous {
        if let Some((_, current_id)) = current
            .iter()
            .find(|(current_value, _)| current_value == value)
        {
            assert_eq!(
                previous_id, current_id,
                "step {step}: node identity for `{attr}={value}` was not preserved"
            );
        }
    }
}
