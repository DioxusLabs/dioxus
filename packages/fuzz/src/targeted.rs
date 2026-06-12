//! Hand-built op recipes that deterministically drive high-value diff paths.
//!
//! Each recipe replays as a test, and can be exported as postcard-encoded
//! corpus seeds via the `write_targeted_seeds_to_corpus` test (gated on
//! `DIOXUS_FUZZ_WRITE_SEEDS=1`).

use crate::case::{FuzzCase, encode_case, run_case};
use crate::model::{
    AttrSpec, AttrValueSpec, DynamicKind, FragmentKeyMode, SuspenseMode, TemplateAttrSpec,
    TemplateNodeKind,
};
use crate::ops::{EventBehaviorSpec, FragmentEdit, ListEdit, Op, TemplateEdit};

#[test]
fn targeted_diff_coverage_cases_replay() {
    for (name, case) in targeted_diff_coverage_cases() {
        run_case(&case).unwrap_or_else(|failure| {
            panic!("targeted diff coverage case {name:?} failed: {failure}")
        });
    }
}

/// Write every targeted diff-coverage case as a postcard-encoded seed file
/// in the libfuzzer corpus directory. The fuzz binary picks these up on
/// the next run so the deterministic high-value patterns we hand-built
/// here also contribute to the coverage-measured fuzz binary, not just
/// the in-process replay test.
///
/// Gated on `DIOXUS_FUZZ_WRITE_SEEDS=1` so a normal `cargo test` doesn't
/// mutate corpus state. Run with:
///   `DIOXUS_FUZZ_WRITE_SEEDS=1 cargo test -p dioxus-vdom-fuzz --lib write_targeted_seeds_to_corpus -- --nocapture`
#[test]
fn write_targeted_seeds_to_corpus() {
    if std::env::var_os("DIOXUS_FUZZ_WRITE_SEEDS").is_none() {
        return;
    }
    let corpus_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fuzz")
        .join("corpus")
        .join("vdom_ops");
    std::fs::create_dir_all(&corpus_dir).unwrap();
    for (name, case) in targeted_diff_coverage_cases() {
        let mut buf = vec![0u8; 64 * 1024];
        let cap = buf.len();
        let len =
            encode_case(&case, &mut buf, cap).unwrap_or_else(|| panic!("failed to encode {name}"));
        let path = corpus_dir.join(format!("targeted-{name}"));
        std::fs::write(&path, &buf[..len])
            .unwrap_or_else(|err| panic!("failed to write seed {name}: {err}"));
        println!("wrote {}", path.display());
    }
}

fn targeted_diff_coverage_cases() -> Vec<(&'static str, FuzzCase)> {
    vec![
        case(
            "non_keyed_append_remove_equal",
            non_keyed_append_remove_equal(),
        ),
        case("keyed_append", keyed_append()),
        case("keyed_prepend", keyed_prepend()),
        case("keyed_remove_and_add_middle", keyed_remove_and_add_middle()),
        case("keyed_replace_all_keys", keyed_replace_all_keys()),
        case("keyed_reorder_insert_remove", keyed_reorder_insert_remove()),
        case("move_static_root", move_root_node_with_kind(None)),
        case(
            "move_dynamic_text_root",
            move_root_node_with_kind(Some(DynamicKind::Text(7))),
        ),
        case(
            "move_dynamic_placeholder_root",
            move_root_node_with_kind(Some(DynamicKind::Placeholder)),
        ),
        case(
            "move_dynamic_fragment_root",
            move_root_node_with_kind(Some(DynamicKind::Fragment {
                children: 1,
                key_base: None,
            })),
        ),
        case(
            "move_dynamic_component_root",
            move_root_node_with_kind(Some(DynamicKind::ComponentA)),
        ),
        case("replace_component_render_fn", replace_component_render_fn()),
        case(
            "hidden_suspense_component_removal",
            hidden_suspense_component_removal(),
        ),
        case("suspense_clear_and_reclaim", suspense_clear_and_reclaim()),
        case(
            "dynamic_attribute_transitions",
            dynamic_attribute_transitions(),
        ),
        case("hidden_suspense_text_diff", {
            vec![
                Op::dynamic(0, 0, suspense_kind(SuspenseMode::Resolved)),
                set_vnode_root_dynamic(1, DynamicKind::Text(0)),
                Op::Rerender,
                Op::suspense(0, SuspenseMode::Ready { wake_after: 0 }),
                Op::Rerender,
                set_vnode_root_dynamic(1, DynamicKind::Text(1)),
                Op::Rerender,
                Op::wake_suspense(0),
            ]
        }),
        case("hidden_suspense_keyed_fragment_diff", {
            vec![
                Op::dynamic(0, 0, suspense_kind(SuspenseMode::Resolved)),
                set_vnode_root_dynamic(
                    1,
                    DynamicKind::Fragment {
                        children: 3,
                        key_base: Some(33),
                    },
                ),
                Op::Rerender,
                Op::suspense(0, SuspenseMode::Ready { wake_after: 0 }),
                Op::Rerender,
                insert_fragment_child(2, Some(36)),
                Op::Rerender,
                Op::wake_suspense(0),
            ]
        }),
        case(
            "dynamic_attribute_static_fallback",
            dynamic_attribute_static_fallback_recipe(),
        ),
        case("portal_create_and_remove", {
            vec![
                set_vnode_root_dynamic(0, DynamicKind::Portal),
                Op::Rerender,
                set_vnode_root_dynamic(0, DynamicKind::Empty),
                Op::Rerender,
            ]
        }),
        case("portal_with_text_child_then_rerender", {
            vec![
                set_vnode_root_dynamic(0, DynamicKind::Portal),
                Op::Rerender,
                set_vnode_root_dynamic(0, DynamicKind::Text(7)),
                Op::Rerender,
            ]
        }),
        case("keyed_portal_text_reorder", {
            // A keyed fragment whose first child is a Portal with a
            // dynamic-text body, reordered so the moved entry is
            // re-pushed via `push_all_root_nodes`. The portal body's
            // text root is mounted in another render target, driving
            // the cross-target `else` arm of the Text case.
            vec![
                set_root_dynamic(),
                fragment_with_children(3, Some(40)),
                set_vnode_root_dynamic(1, DynamicKind::Portal),
                set_vnode_root_dynamic(2, DynamicKind::Text(7)),
                set_vnode_root_dynamic(3, DynamicKind::Portal),
                set_vnode_root_dynamic(4, DynamicKind::Text(8)),
                set_vnode_root_dynamic(5, DynamicKind::Portal),
                set_vnode_root_dynamic(6, DynamicKind::Text(9)),
                Op::Rerender,
                move_fragment_child(0, 2),
                move_fragment_child(0, 1),
                Op::Rerender,
            ]
        }),
        case("portal_inside_fragment_and_remove", {
            vec![
                set_root_dynamic(),
                insert_fragment_child(0, None),
                Op::Rerender,
                set_vnode_root_dynamic(1, DynamicKind::Portal),
                Op::Rerender,
                remove_fragment_child(0),
                Op::Rerender,
            ]
        }),
        case("unkeyed_fragment_batched_component_props_diff", {
            // Build an unkeyed fragment whose children are all
            // ComponentA. With >FRAGMENT_WORK_BATCH (16) same-component
            // children, `diff_child_pairs` exercises the batched
            // `queue_component_props_diff` path in iterator.rs.
            let mut ops = vec![set_root_dynamic()];
            for _ in 0..18 {
                ops.push(insert_fragment_child(0, None));
            }
            for vnode in 1..=18u8 {
                ops.push(set_vnode_root_dynamic(vnode, DynamicKind::ComponentA));
            }
            ops.push(Op::Rerender);
            // Mark the root dirty so the unkeyed fragment re-diffs all
            // children (not just creates them).
            ops.push(insert_fragment_child(0, None));
            ops.push(Op::Rerender);
            ops
        }),
        case("scope_event_behaviors", scope_event_behaviors()),
        case(
            "hidden_suspense_component_dirty_task",
            hidden_suspense_component_dirty_task(),
        ),
        case(
            "hidden_suspense_component_suspense_immediate",
            hidden_suspense_component_suspense_immediate(),
        ),
        case(
            "nonsuspense_scope_suspense_immediate",
            nonsuspense_scope_suspense_immediate(),
        ),
    ]
}

fn case(name: &'static str, ops: Vec<Op>) -> (&'static str, FuzzCase) {
    (name, FuzzCase::new(ops))
}

fn suspense_kind(mode: SuspenseMode) -> DynamicKind {
    DynamicKind::Suspense { mode }
}

fn set_root_dynamic() -> Op {
    Op::template(
        0,
        TemplateEdit::SetNode {
            node: 0,
            kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
        },
    )
}

fn insert_fragment_child(index: u8, key: Option<u8>) -> Op {
    Op::fragment(
        0,
        0,
        FragmentEdit::Children(ListEdit::Insert { index, item: key }),
    )
}

fn remove_fragment_child(index: u8) -> Op {
    Op::fragment(0, 0, FragmentEdit::Children(ListEdit::Remove { index }))
}

fn move_fragment_child(from: u8, to: u8) -> Op {
    Op::fragment(0, 0, FragmentEdit::Children(ListEdit::Move { from, to }))
}

fn key_fragment(base: u8) -> Op {
    Op::fragment(0, 0, FragmentEdit::KeyMode(FragmentKeyMode::Keyed { base }))
}

fn fragment_with_children(count: u8, key_base: Option<u8>) -> Op {
    Op::dynamic(
        0,
        0,
        DynamicKind::Fragment {
            children: count,
            key_base,
        },
    )
}

fn set_vnode_root_dynamic(vnode: u8, kind: DynamicKind) -> Op {
    Op::template(
        vnode,
        TemplateEdit::SetNode {
            node: 0,
            kind: TemplateNodeKind::Dynamic(kind),
        },
    )
}

fn mount_root_listener() -> Vec<Op> {
    vec![
        Op::template(
            0,
            TemplateEdit::Attrs {
                element: 0,
                edit: ListEdit::Insert {
                    index: 0,
                    item: TemplateAttrSpec::Dynamic(Vec::new()),
                },
            },
        ),
        Op::dynamic_attrs(
            0,
            0,
            ListEdit::Insert {
                index: 0,
                item: AttrSpec {
                    name: 1,
                    namespace: None,
                    value: AttrValueSpec::Listener,
                    volatile: false,
                },
            },
        ),
        Op::Rerender,
    ]
}

fn scope_event_behaviors() -> Vec<Op> {
    let mut ops = mount_root_listener();
    ops.extend([
        Op::fire_event(0, EventBehaviorSpec::ContextRoundTrip),
        Op::fire_event(0, EventBehaviorSpec::RootContextRoundTrip),
        Op::fire_event(0, EventBehaviorSpec::QueueEffect),
        Op::fire_event(0, EventBehaviorSpec::SpawnIsomorphic),
        Op::fire_event(0, EventBehaviorSpec::ScheduleUpdate),
        Op::fire_event(0, EventBehaviorSpec::ScheduleUpdateAny),
        Op::fire_event(0, EventBehaviorSpec::NeedsUpdate),
        Op::fire_event(0, EventBehaviorSpec::NeedsUpdateAny),
    ]);
    ops
}

fn hidden_suspense_component_dirty_task() -> Vec<Op> {
    vec![
        set_root_dynamic(),
        Op::dynamic(0, 0, suspense_kind(SuspenseMode::Pending)),
        set_vnode_root_dynamic(1, DynamicKind::ComponentA),
        Op::Rerender,
        Op::RenderDirty,
    ]
}

fn hidden_suspense_component_suspense_immediate() -> Vec<Op> {
    vec![
        set_root_dynamic(),
        Op::dynamic(0, 0, suspense_kind(SuspenseMode::Pending)),
        set_vnode_root_dynamic(1, DynamicKind::ComponentA),
        Op::Rerender,
        Op::RenderSuspenseDirty,
    ]
}

fn nonsuspense_scope_suspense_immediate() -> Vec<Op> {
    vec![
        Op::template(
            0,
            TemplateEdit::SetNode {
                node: 0,
                kind: TemplateNodeKind::Text(42),
            },
        ),
        Op::RenderSuspenseDirty,
    ]
}

fn non_keyed_append_remove_equal() -> Vec<Op> {
    vec![
        set_root_dynamic(),
        insert_fragment_child(0, None),
        insert_fragment_child(1, None),
        Op::Rerender,
        insert_fragment_child(2, None),
        Op::Rerender,
        remove_fragment_child(1),
        Op::Rerender,
        Op::template(
            1,
            TemplateEdit::SetNode {
                node: 0,
                kind: TemplateNodeKind::Text(11),
            },
        ),
        Op::Rerender,
    ]
}

fn keyed_append() -> Vec<Op> {
    vec![
        set_root_dynamic(),
        fragment_with_children(2, Some(0)),
        Op::Rerender,
        insert_fragment_child(2, Some(2)),
        Op::Rerender,
    ]
}

fn keyed_prepend() -> Vec<Op> {
    vec![
        set_root_dynamic(),
        fragment_with_children(2, Some(1)),
        Op::Rerender,
        insert_fragment_child(0, Some(0)),
        Op::Rerender,
    ]
}

fn keyed_remove_and_add_middle() -> Vec<Op> {
    vec![
        set_root_dynamic(),
        fragment_with_children(3, Some(0)),
        Op::Rerender,
        remove_fragment_child(1),
        Op::Rerender,
        insert_fragment_child(1, Some(1)),
        Op::Rerender,
    ]
}

fn keyed_replace_all_keys() -> Vec<Op> {
    vec![
        set_root_dynamic(),
        fragment_with_children(2, Some(0)),
        Op::Rerender,
        key_fragment(2),
        Op::Rerender,
    ]
}

fn keyed_reorder_insert_remove() -> Vec<Op> {
    vec![
        set_root_dynamic(),
        fragment_with_children(5, Some(0)),
        Op::Rerender,
        move_fragment_child(3, 1),
        insert_fragment_child(2, Some(5)),
        remove_fragment_child(4),
        Op::Rerender,
    ]
}

fn move_root_node_with_kind(kind: Option<DynamicKind>) -> Vec<Op> {
    let mut ops = vec![set_root_dynamic(), fragment_with_children(4, Some(0))];

    if let Some(kind) = kind {
        // The child vnode selected here must materialize its nested content
        // before the keyed move so push_all_root_nodes has live roots to
        // collect.
        ops.push(set_vnode_root_dynamic(3, kind));
    }

    ops.extend([Op::Rerender, move_fragment_child(2, 0), Op::Rerender]);
    ops
}

fn replace_component_render_fn() -> Vec<Op> {
    vec![
        set_root_dynamic(),
        Op::dynamic(0, 0, DynamicKind::ComponentA),
        Op::Rerender,
        Op::dynamic(0, 0, DynamicKind::ComponentB),
        Op::Rerender,
    ]
}

fn hidden_suspense_component_removal() -> Vec<Op> {
    vec![
        set_root_dynamic(),
        Op::dynamic(0, 0, suspense_kind(SuspenseMode::Resolved)),
        Op::template(
            1,
            TemplateEdit::Children {
                element: 0,
                edit: ListEdit::Insert {
                    index: 0,
                    item: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                },
            },
        ),
        Op::template(
            1,
            TemplateEdit::Children {
                element: 0,
                edit: ListEdit::Insert {
                    index: 1,
                    item: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                },
            },
        ),
        Op::dynamic(1, 0, suspense_kind(SuspenseMode::Pending)),
        Op::dynamic(1, 1, DynamicKind::ComponentA),
        Op::Rerender,
        Op::template(
            1,
            TemplateEdit::Children {
                element: 0,
                edit: ListEdit::Remove { index: 1 },
            },
        ),
        Op::Rerender,
    ]
}

fn suspense_clear_and_reclaim() -> Vec<Op> {
    vec![
        set_root_dynamic(),
        Op::dynamic(0, 0, suspense_kind(SuspenseMode::Ready { wake_after: 0 })),
        set_vnode_root_dynamic(1, DynamicKind::Empty),
        Op::Rerender,
        Op::wake_suspense(0),
        Op::dynamic(1, 0, DynamicKind::ComponentA),
        Op::Rerender,
        Op::suspense(0, SuspenseMode::Ready { wake_after: 0 }),
        Op::Rerender,
        Op::wake_suspense(0),
    ]
}

fn dynamic_attribute_transitions() -> Vec<Op> {
    vec![
        Op::template(
            0,
            TemplateEdit::Attrs {
                element: 0,
                edit: ListEdit::Insert {
                    index: 0,
                    item: TemplateAttrSpec::Static {
                        name: 9,
                        value: 1,
                        namespace: None,
                    },
                },
            },
        ),
        Op::template(
            0,
            TemplateEdit::Attrs {
                element: 0,
                edit: ListEdit::Insert {
                    index: 1,
                    item: TemplateAttrSpec::Dynamic(Vec::new()),
                },
            },
        ),
        Op::dynamic_attrs(
            0,
            0,
            ListEdit::Insert {
                index: 0,
                item: AttrSpec {
                    name: 9,
                    namespace: None,
                    value: AttrValueSpec::Text(1),
                    volatile: false,
                },
            },
        ),
        Op::Rerender,
        Op::dynamic_attrs(
            0,
            0,
            ListEdit::Insert {
                index: 0,
                item: AttrSpec {
                    name: 9,
                    namespace: None,
                    value: AttrValueSpec::None,
                    volatile: true,
                },
            },
        ),
        Op::Rerender,
        Op::dynamic_attrs(
            0,
            0,
            ListEdit::Insert {
                index: 0,
                item: AttrSpec {
                    name: 1,
                    namespace: None,
                    value: AttrValueSpec::Listener,
                    volatile: false,
                },
            },
        ),
        Op::Rerender,
        Op::dynamic_attrs(
            0,
            0,
            ListEdit::Insert {
                index: 0,
                item: AttrSpec {
                    name: 1,
                    namespace: None,
                    value: AttrValueSpec::Text(2),
                    volatile: false,
                },
            },
        ),
        Op::Rerender,
    ]
}

fn dynamic_attribute_static_fallback_recipe() -> Vec<Op> {
    vec![
        Op::template(
            0,
            TemplateEdit::Attrs {
                element: 0,
                edit: ListEdit::Insert {
                    index: 0,
                    item: TemplateAttrSpec::Static {
                        name: 33,
                        value: 129,
                        namespace: None,
                    },
                },
            },
        ),
        Op::template(
            0,
            TemplateEdit::Attrs {
                element: 0,
                edit: ListEdit::Insert {
                    index: 1,
                    item: TemplateAttrSpec::Dynamic(Vec::new()),
                },
            },
        ),
        Op::dynamic_attrs(
            0,
            0,
            ListEdit::Insert {
                index: 0,
                item: AttrSpec {
                    name: 33,
                    namespace: None,
                    value: AttrValueSpec::Text(2),
                    volatile: false,
                },
            },
        ),
        Op::Rerender,
        Op::dynamic_attrs(0, 0, ListEdit::Remove { index: 0 }),
        Op::Rerender,
        Op::dynamic_attrs(
            0,
            0,
            ListEdit::Insert {
                index: 0,
                item: AttrSpec {
                    name: 33,
                    namespace: None,
                    value: AttrValueSpec::Text(1),
                    volatile: false,
                },
            },
        ),
        Op::Rerender,
        Op::dynamic_attrs(0, 0, ListEdit::Remove { index: 0 }),
        Op::Rerender,
        Op::dynamic_attrs(
            0,
            0,
            ListEdit::Insert {
                index: 0,
                item: AttrSpec {
                    name: 33,
                    namespace: None,
                    value: AttrValueSpec::Int(3),
                    volatile: false,
                },
            },
        ),
        Op::Rerender,
        Op::dynamic_attrs(0, 0, ListEdit::Remove { index: 0 }),
        Op::dynamic_attrs(
            0,
            0,
            ListEdit::Insert {
                index: 0,
                item: AttrSpec {
                    name: 33,
                    namespace: None,
                    value: AttrValueSpec::Bool(true),
                    volatile: false,
                },
            },
        ),
        Op::Rerender,
    ]
}
