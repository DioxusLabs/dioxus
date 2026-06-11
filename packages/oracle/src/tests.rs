use super::*;
use dioxus::prelude::*;

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
fn records_historical_event_listener_targets() {
    let seen_id = std::rc::Rc::new(std::cell::Cell::new(None));
    Sequence::new()
        .step_with(|| {
            rsx! {
                button { onclick: move |_| {}, "go" }
            }
        })
        .interlude({
            let seen_id = seen_id.clone();
            move |_, oracle| {
                let id = oracle.element_id_by_tag("button");
                seen_id.set(Some(id));
                assert_eq!(
                    oracle.historical_event_listener_targets(),
                    &[EventListenerTarget { name: "click", id }]
                );
            }
        })
        .step(rsx! {
            button { "go" }
        })
        .interlude({
            let seen_id = seen_id.clone();
            move |_, oracle| {
                let id = seen_id.get().expect("listener id should be captured");
                assert_eq!(
                    oracle.historical_event_listener_targets(),
                    &[EventListenerTarget { name: "click", id }]
                );
            }
        })
        .run();
}

#[test]
fn keeps_historical_event_listener_targets_after_node_removal() {
    let seen_id = std::rc::Rc::new(std::cell::Cell::new(None));
    Sequence::new()
        .step_with(|| {
            rsx! {
                button { onclick: move |_| {}, "go" }
            }
        })
        .interlude({
            let seen_id = seen_id.clone();
            move |_, oracle| {
                seen_id.set(Some(oracle.element_id_by_tag("button")));
            }
        })
        .step(rsx! {
            div { "gone" }
        })
        .interlude({
            let seen_id = seen_id.clone();
            move |_, oracle| {
                let id = seen_id.get().expect("listener id should be captured");
                assert_eq!(
                    oracle.historical_event_listener_targets(),
                    &[EventListenerTarget { name: "click", id }]
                );
            }
        })
        .run();
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
    renderer.rebuild(&mut vdom);
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
fn sequence_walks_states_in_order() {
    Sequence::new()
        .step(rsx! { div { "a" } })
        .step(rsx! { div { "b" } })
        .step(rsx! { div { "c" } })
        .run();
}

#[test]
fn sequence_tracks_identity_for_moved_nodes() {
    fn divs(keys: &[i32]) -> Element {
        rsx! {
            for k in keys.iter().copied() {
                div { key: "{k}", id: "{k}", "{k}" }
            }
        }
    }
    // Reordering keyed nodes should *move* DOM nodes — identities preserved.
    Sequence::new()
        .track_identity_by("id")
        .step(divs(&[0, 1, 2, 3]))
        .step(divs(&[3, 0, 1, 2]))
        .step(divs(&[2, 3, 0, 1]))
        .run();
}

#[test]
fn sequence_runs_interlude_between_steps() {
    use std::cell::Cell;
    thread_local! {
        static CALLS: Cell<usize> = const { Cell::new(0) };
    }
    CALLS.with(|c| c.set(0));
    Sequence::new()
        .step(rsx! { div { "a" } })
        .interlude(|_dom, _oracle| {
            CALLS.with(|c| c.set(c.get() + 1));
        })
        .step(rsx! { div { "b" } })
        .interlude(|_dom, _oracle| {
            CALLS.with(|c| c.set(c.get() + 1));
        })
        .step(rsx! { div { "c" } })
        .run();
    assert_eq!(CALLS.with(|c| c.get()), 2);
}

#[test]
#[should_panic(expected = "node identity for `id=hot` was not preserved")]
fn sequence_identity_check_catches_recreation() {
    // Two unkeyed elements of different tag — the diff has to drop the old
    // node and create a new one. The identity tracker catches that.
    Sequence::new()
        .track_identity_by("id")
        .step(rsx! { div { id: "hot", "before" } })
        .step(rsx! { span { id: "hot", "after" } })
        .run();
}

#[test]
fn edit_summary_counts_rebuild_then_in_place_patch() {
    // First step builds the tree; rerender with the same shape but a
    // different *dynamic* text body should patch in place — same template,
    // just a new value for the dynamic slot.
    fn body(value: &str) -> Element {
        rsx! { div { id: "0", "{value}" } }
    }
    Sequence::new()
        .step(body("alpha"))
        .step(body("beta"))
        .assert_edit_summary(0, |s| {
            assert!(s.loads >= 1, "rebuild should load at least one template");
        })
        .assert_edit_summary(1, |s| {
            assert_eq!(s.loads, 0, "in-place text patch should not load templates");
            assert_eq!(s.set_texts, 1, "exactly one text patch expected");
            assert_eq!(s.removes, 0);
            assert_eq!(s.replaces, 0);
        })
        .run();
}

#[test]
#[should_panic(expected = "expected one move")]
fn edit_summary_assertion_fires_on_failure() {
    // Force the assertion to fail to confirm panics propagate.
    Sequence::new()
        .step(rsx! { div { id: "0" } })
        .step(rsx! { div { id: "0", "x" } })
        .assert_edit_summary(1, |_| panic!("expected one move"))
        .run();
}

#[test]
#[should_panic(expected = "references step 5 but the sequence only has 2 step")]
fn edit_summary_assertion_step_out_of_range() {
    Sequence::new()
        .step(rsx! { div {} })
        .step(rsx! { div {} })
        .assert_edit_summary(5, |_| {})
        .run();
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
