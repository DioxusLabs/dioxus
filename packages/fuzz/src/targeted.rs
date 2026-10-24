//! Hand-built op recipes that deterministically drive high-value diff paths.
//!
//! Each recipe replays as a test, and can be exported as postcard-encoded
//! corpus seeds via the `write_targeted_seeds_to_corpus` test (gated on
//! `DIOXUS_FUZZ_WRITE_SEEDS=1`).

use crate::case::{FuzzCase, encode_case};
use crate::harness::{Harness, apply_step};
use crate::model::{
    AttrSpec, AttrValueSpec, DynamicKind, FragmentKeyMode, SuspenseMode, TemplateAttrSpec,
    TemplateNodeKind,
};
use crate::ops::{EventBehaviorSpec, FragmentEdit, ListEdit, Op, TemplateEdit};

#[test]
fn targeted_diff_coverage_cases_replay() {
    for (name, case) in targeted_diff_coverage_cases() {
        replay_case_strict(name, &case);
    }
}

/// Regression: two adjacent root-level dynamic nodes lower to a single
/// append-at-root anchor (`{a}{b}`). When the first slot becomes a pending
/// suspense and re-renders, its fallback must be placed before the live sibling
/// value sharing that anchor - not appended to the document root, which left
/// the two roots in swapped order versus a fresh build. Minimized libfuzzer
/// artifact for that divergence.
#[test]
fn suspense_root_sibling_placement_keeps_order() {
    let bytes: &[u8] = &[
        5, 3, 0, 0, 1, 0, 0, 2, 0, 3, 0, 0, 0, 1, 2, 3, 5, 3, 0, 0, 0, 178, 2, 5, 1, 5,
    ];
    let case = crate::case::decode_case(bytes).expect("decode regression artifact");
    replay_case_strict("suspense_root_sibling_placement", &case);
}

/// Regression: an unkeyed fragment grows from one child to several, and the
/// pre-existing first child has no live DOM (its sole root is an empty
/// fragment). The new tail children must be ordered after that child's own
/// content, not appended to the document root: anchoring the tail on the empty
/// old child found no insertion edge and appended, then the child's replacement
/// content landed last, swapping the roots versus a fresh build. Minimized
/// libfuzzer artifact for that divergence.
#[test]
fn unkeyed_tail_insert_past_empty_leading_child_keeps_order() {
    let bytes: &[u8] = &[
        7, 3, 0, 0, 0, 0, 2, 0, 3, 0, 0, 4, 0, 1, 0, 0, 0, 3, 0, 1, 0, 0, 2, 0, 0, 3, 0, 1, 1, 0,
        0, 2, 1, 120, 3, 0, 0, 0, 0, 2, 2, 3, 0, 0,
    ];
    let case = crate::case::decode_case(bytes).expect("decode regression artifact");
    replay_case_strict("unkeyed_tail_insert_past_empty_leading_child", &case);
}

fn replay_case_strict(name: &str, case: &FuzzCase) {
    let mut state = Harness::fresh_strict();
    for (step, op) in case.ops.iter().enumerate() {
        apply_step(&mut state, op).unwrap_or_else(|failure| {
            panic!(
                "targeted diff coverage case {name:?} failed at step {step} while applying {op:?}: {failure}"
            )
        });
    }
}

/// Replay the entire on-disk libfuzzer corpus through the diff oracle in-process
/// so a coverage build exercises every diff path the fuzzer has explored, not
/// just the curated regression cases above. Gated on `DIOXUS_FUZZ_REPLAY_CORPUS=1`
/// because replaying the full corpus is slow; enable it for coverage runs:
///   `DIOXUS_FUZZ_REPLAY_CORPUS=1 cargo llvm-cov ... -p dioxus-vdom-fuzz`
#[test]
fn replay_corpus_for_coverage() {
    if std::env::var_os("DIOXUS_FUZZ_REPLAY_CORPUS").is_none() {
        return;
    }
    let corpus_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fuzz")
        .join("corpus")
        .join("vdom_ops");
    for entry in std::fs::read_dir(&corpus_dir)
        .unwrap_or_else(|err| panic!("read corpus dir {}: {err}", corpus_dir.display()))
    {
        let path = entry.expect("corpus entry").path();
        if !path.is_file() {
            continue;
        }
        let bytes = std::fs::read(&path).expect("read corpus file");
        if let Some(case) = crate::case::decode_case(&bytes) {
            // Coverage replay: drive the diff paths. Oracle divergences are the
            // fuzzer's job to catch, so ignore the comparison result here.
            let _ = crate::case::run_case(&case);
        }
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
            "dynamic_attribute_static_restore",
            dynamic_attribute_static_restore_recipe(),
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
                keyed_fragment_with_children(3, 40),
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
        case("keyed_portal_static_root_reorder", {
            // Same cross-target push path as the dynamic-text portal case,
            // but with the portal body left as its default static root. That
            // proves static roots only push when their mount target matches
            // the active renderer target.
            vec![
                set_root_dynamic(),
                keyed_fragment_with_children(3, 50),
                set_vnode_root_dynamic(1, DynamicKind::Portal),
                set_vnode_root_dynamic(3, DynamicKind::Portal),
                set_vnode_root_dynamic(5, DynamicKind::Portal),
                Op::Rerender,
                move_fragment_child(2, 0),
                move_fragment_child(2, 1),
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
        case(
            "hidden_suspense_portal_diff_without_writer",
            hidden_suspense_portal_diff_without_writer(),
        ),
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
        case("nested_dynamic_attr_values", nested_dynamic_attr_values()),
        case(
            "component_replace_without_live_dom",
            component_replace_without_live_dom(),
        ),
        case(
            "hidden_suspense_fragment_growth_without_writer",
            hidden_suspense_fragment_growth_without_writer(),
        ),
        case(
            "hidden_suspense_static_root_replace_without_writer",
            hidden_suspense_static_root_replace_without_writer(),
        ),
        case(
            "visible_root_text_replacement",
            visible_root_text_replacement(),
        ),
        case(
            "non_root_dynamic_attr_reclaim_on_template_replace",
            non_root_dynamic_attr_reclaim_on_template_replace(),
        ),
        case("visible_adjacent_text_slot", visible_adjacent_text_slot()),
        case(
            "visible_adjacent_component_slot",
            visible_adjacent_component_slot(),
        ),
        case("visible_deep_slot_insert", visible_deep_slot_insert()),
        case(
            "visible_different_parent_dynamic_scan",
            visible_different_parent_dynamic_scan(),
        ),
        case(
            "visible_middle_slot_skips_empty_before_later_text",
            visible_middle_slot_skips_empty_before_later_text(),
        ),
        case(
            "visible_fragment_child_after_text_slot",
            visible_fragment_child_after_text_slot(),
        ),
        case(
            "dynamic_before_non_first_static_with_live_sibling",
            dynamic_before_non_first_static_with_live_sibling(),
        ),
        case(
            "root_dynamic_before_static_root_with_nested_dynamic_node",
            root_dynamic_before_static_root_with_nested_dynamic_node(),
        ),
        case(
            "root_dynamic_before_static_root_with_nested_dynamic_attr",
            root_dynamic_before_static_root_with_nested_dynamic_attr(),
        ),
        case(
            "sibling_dynamic_listener_event",
            sibling_dynamic_listener_event(),
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
    insert_fragment_child_in(0, 0, index, key)
}

fn insert_fragment_child_in(vnode: u8, slot: u8, index: u8, key: Option<u8>) -> Op {
    Op::fragment(
        vnode,
        slot,
        FragmentEdit::Children(ListEdit::Insert { index, item: key }),
    )
}

fn remove_fragment_child(index: u8) -> Op {
    Op::fragment(0, 0, FragmentEdit::Children(ListEdit::Remove { index }))
}

fn move_fragment_child(from: u8, to: u8) -> Op {
    Op::fragment(0, 0, FragmentEdit::Children(ListEdit::Move { from, to }))
}

fn keyed_fragment_with_children(count: u8, key_base: u8) -> Op {
    Op::dynamic(
        0,
        0,
        DynamicKind::Fragment {
            children: count,
            key_base: Some(key_base),
        },
    )
}

fn set_vnode_root_dynamic(vnode: u8, kind: DynamicKind) -> Op {
    set_vnode_node(vnode, 0, TemplateNodeKind::Dynamic(kind))
}

fn set_vnode_node(vnode: u8, node: u8, kind: TemplateNodeKind) -> Op {
    Op::template(vnode, TemplateEdit::SetNode { node, kind })
}

fn insert_element_root(vnode: u8, index: u8, tag: u8) -> Op {
    Op::template(
        vnode,
        TemplateEdit::Roots {
            edit: ListEdit::Insert {
                index,
                item: TemplateNodeKind::Element {
                    tag,
                    namespace: None,
                },
            },
        },
    )
}

fn insert_child(vnode: u8, element: u8, index: u8, kind: TemplateNodeKind) -> Op {
    Op::template(
        vnode,
        TemplateEdit::Children {
            element,
            edit: ListEdit::Insert { index, item: kind },
        },
    )
}

fn insert_dynamic_attr_slot(vnode: u8, element: u8, index: u8) -> Op {
    Op::template(
        vnode,
        TemplateEdit::Attrs {
            element,
            edit: ListEdit::Insert {
                index,
                item: TemplateAttrSpec::Dynamic(Vec::new()),
            },
        },
    )
}

fn insert_dynamic_attr(vnode: u8, attr: u8, index: u8, value: AttrValueSpec) -> Op {
    Op::dynamic_attrs(
        vnode,
        attr,
        ListEdit::Insert {
            index,
            item: AttrSpec {
                name: 5,
                namespace: None,
                value,
                volatile: false,
            },
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
        keyed_fragment_with_children(2, 0),
        Op::Rerender,
        insert_fragment_child(2, Some(2)),
        Op::Rerender,
    ]
}

fn keyed_prepend() -> Vec<Op> {
    vec![
        set_root_dynamic(),
        keyed_fragment_with_children(2, 1),
        Op::Rerender,
        insert_fragment_child(0, Some(0)),
        Op::Rerender,
    ]
}

fn keyed_remove_and_add_middle() -> Vec<Op> {
    vec![
        set_root_dynamic(),
        keyed_fragment_with_children(3, 0),
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
        keyed_fragment_with_children(2, 0),
        Op::Rerender,
        Op::fragment(
            0,
            0,
            FragmentEdit::KeyMode(FragmentKeyMode::Keyed { base: 2 }),
        ),
        Op::Rerender,
    ]
}

fn keyed_reorder_insert_remove() -> Vec<Op> {
    vec![
        set_root_dynamic(),
        keyed_fragment_with_children(5, 0),
        Op::Rerender,
        move_fragment_child(3, 1),
        insert_fragment_child(2, Some(5)),
        remove_fragment_child(4),
        Op::Rerender,
    ]
}

fn move_root_node_with_kind(kind: Option<DynamicKind>) -> Vec<Op> {
    let mut ops = vec![set_root_dynamic(), keyed_fragment_with_children(4, 0)];

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

fn dynamic_attribute_static_restore_recipe() -> Vec<Op> {
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

fn nested_dynamic_attr_values() -> Vec<Op> {
    vec![
        insert_child(
            0,
            0,
            0,
            TemplateNodeKind::Element {
                tag: 1,
                namespace: None,
            },
        ),
        insert_dynamic_attr_slot(0, 1, 0),
        insert_dynamic_attr_slot(0, 0, 0),
        insert_dynamic_attr(0, 0, 0, AttrValueSpec::Float(7)),
        insert_dynamic_attr(0, 1, 0, AttrValueSpec::Listener),
        Op::Rerender,
        insert_dynamic_attr(0, 0, 0, AttrValueSpec::Float(7)),
        Op::Rerender,
        insert_dynamic_attr(0, 0, 0, AttrValueSpec::Int(9)),
        Op::Rerender,
        insert_dynamic_attr(0, 0, 0, AttrValueSpec::Int(9)),
        Op::Rerender,
        insert_dynamic_attr(0, 0, 0, AttrValueSpec::Bool(true)),
        Op::Rerender,
        insert_dynamic_attr(0, 0, 0, AttrValueSpec::Bool(true)),
        Op::Rerender,
        insert_dynamic_attr(0, 0, 0, AttrValueSpec::Any(11)),
        Op::Rerender,
        insert_dynamic_attr(0, 0, 0, AttrValueSpec::Any(11)),
        Op::Rerender,
        Op::fire_event(0, EventBehaviorSpec::Noop),
        Op::template(
            0,
            TemplateEdit::Children {
                element: 0,
                edit: ListEdit::Remove { index: 0 },
            },
        ),
        Op::Rerender,
    ]
}

fn component_replace_without_live_dom() -> Vec<Op> {
    vec![
        set_root_dynamic(),
        Op::dynamic(0, 0, DynamicKind::ComponentA),
        set_vnode_root_dynamic(1, DynamicKind::Empty),
        Op::Rerender,
        Op::dynamic(0, 0, DynamicKind::ComponentB),
        Op::Rerender,
    ]
}

fn hidden_suspense_fragment_growth_without_writer() -> Vec<Op> {
    vec![
        set_root_dynamic(),
        Op::dynamic(0, 0, suspense_kind(SuspenseMode::Resolved)),
        set_vnode_root_dynamic(
            1,
            DynamicKind::Fragment {
                children: 0,
                key_base: None,
            },
        ),
        Op::Rerender,
        Op::suspense(0, SuspenseMode::Pending),
        Op::Rerender,
        set_vnode_root_dynamic(
            1,
            DynamicKind::Fragment {
                children: 1,
                key_base: None,
            },
        ),
        Op::Rerender,
        insert_fragment_child_in(1, 0, 1, None),
        Op::Rerender,
    ]
}

fn hidden_suspense_static_root_replace_without_writer() -> Vec<Op> {
    vec![
        set_root_dynamic(),
        Op::dynamic(0, 0, suspense_kind(SuspenseMode::Pending)),
        insert_child(1, 0, 0, TemplateNodeKind::Dynamic(DynamicKind::Text(1))),
        Op::Rerender,
        set_vnode_root_dynamic(1, DynamicKind::Text(2)),
        Op::Rerender,
    ]
}

fn visible_root_text_replacement() -> Vec<Op> {
    vec![
        set_root_dynamic(),
        Op::dynamic(0, 0, DynamicKind::Text(1)),
        Op::Rerender,
        Op::dynamic(0, 0, DynamicKind::Empty),
        Op::Rerender,
    ]
}

fn non_root_dynamic_attr_reclaim_on_template_replace() -> Vec<Op> {
    vec![
        insert_child(
            0,
            0,
            0,
            TemplateNodeKind::Element {
                tag: 1,
                namespace: None,
            },
        ),
        insert_dynamic_attr_slot(0, 1, 0),
        insert_dynamic_attr_slot(0, 1, 1),
        insert_dynamic_attr(0, 0, 0, AttrValueSpec::Text(4)),
        insert_dynamic_attr(0, 1, 0, AttrValueSpec::Text(5)),
        Op::Rerender,
        set_vnode_node(0, 0, TemplateNodeKind::Text(9)),
        Op::Rerender,
    ]
}

fn hidden_suspense_portal_diff_without_writer() -> Vec<Op> {
    vec![
        set_root_dynamic(),
        Op::dynamic(0, 0, suspense_kind(SuspenseMode::Resolved)),
        set_vnode_root_dynamic(1, DynamicKind::Portal),
        set_vnode_root_dynamic(2, DynamicKind::Text(0)),
        Op::Rerender,
        Op::suspense(0, SuspenseMode::Pending),
        Op::Rerender,
        set_vnode_root_dynamic(2, DynamicKind::Text(1)),
        Op::Rerender,
    ]
}

fn visible_adjacent_text_slot() -> Vec<Op> {
    vec![
        insert_child(0, 0, 0, TemplateNodeKind::Dynamic(DynamicKind::Empty)),
        insert_child(0, 0, 1, TemplateNodeKind::Dynamic(DynamicKind::Text(2))),
        Op::Rerender,
        set_vnode_node(0, 1, TemplateNodeKind::Dynamic(DynamicKind::Text(1))),
        Op::Rerender,
    ]
}

fn visible_adjacent_component_slot() -> Vec<Op> {
    vec![
        insert_child(0, 0, 0, TemplateNodeKind::Dynamic(DynamicKind::Empty)),
        insert_child(0, 0, 1, TemplateNodeKind::Dynamic(DynamicKind::ComponentA)),
        Op::Rerender,
        set_vnode_node(0, 1, TemplateNodeKind::Dynamic(DynamicKind::Text(1))),
        Op::Rerender,
    ]
}

fn visible_deep_slot_insert() -> Vec<Op> {
    vec![
        insert_child(
            0,
            0,
            0,
            TemplateNodeKind::Element {
                tag: 1,
                namespace: None,
            },
        ),
        insert_child(0, 1, 0, TemplateNodeKind::Dynamic(DynamicKind::Empty)),
        insert_child(0, 1, 1, TemplateNodeKind::Text(7)),
        Op::Rerender,
        set_vnode_node(0, 2, TemplateNodeKind::Dynamic(DynamicKind::Text(1))),
        Op::Rerender,
    ]
}

fn visible_different_parent_dynamic_scan() -> Vec<Op> {
    vec![
        insert_child(
            0,
            0,
            0,
            TemplateNodeKind::Element {
                tag: 1,
                namespace: None,
            },
        ),
        insert_child(0, 1, 0, TemplateNodeKind::Dynamic(DynamicKind::Empty)),
        insert_child(
            0,
            0,
            1,
            TemplateNodeKind::Element {
                tag: 2,
                namespace: None,
            },
        ),
        insert_child(0, 2, 0, TemplateNodeKind::Dynamic(DynamicKind::Text(2))),
        Op::Rerender,
        set_vnode_node(0, 2, TemplateNodeKind::Dynamic(DynamicKind::Text(1))),
        Op::Rerender,
    ]
}

fn visible_middle_slot_skips_empty_before_later_text() -> Vec<Op> {
    vec![
        insert_child(0, 0, 0, TemplateNodeKind::Dynamic(DynamicKind::Text(0))),
        insert_child(0, 0, 1, TemplateNodeKind::Dynamic(DynamicKind::Empty)),
        insert_child(0, 0, 2, TemplateNodeKind::Dynamic(DynamicKind::Empty)),
        insert_child(0, 0, 3, TemplateNodeKind::Dynamic(DynamicKind::Text(2))),
        Op::Rerender,
        set_vnode_node(0, 2, TemplateNodeKind::Dynamic(DynamicKind::Text(1))),
        Op::Rerender,
    ]
}

fn visible_fragment_child_after_text_slot() -> Vec<Op> {
    vec![
        insert_child(0, 0, 0, TemplateNodeKind::Dynamic(DynamicKind::Text(0))),
        insert_child(
            0,
            0,
            1,
            TemplateNodeKind::Dynamic(DynamicKind::Fragment {
                children: 1,
                key_base: None,
            }),
        ),
        set_vnode_root_dynamic(1, DynamicKind::Empty),
        Op::Rerender,
        set_vnode_root_dynamic(1, DynamicKind::Text(1)),
        Op::Rerender,
    ]
}

/// Regression: a dynamic node placed before a *non-first* static sibling must
/// land at the correct live position when an earlier dynamic sibling is already
/// materialized in the DOM.
///
/// Builds `div { span {a} span {b=Empty} span }`. The first render leaves `{b}`
/// empty, so `{a}` is the only live dynamic node. Toggling `{b}` to text then
/// anchors the insertion on a *static* child index (counting only static
/// children), but the live DOM child index is shifted by the already-live `{a}`,
/// so `{b}` was inserted before the wrong `span`. Mirrors the core test
/// `dynamic_node_before_non_first_static_sibling_keeps_order`; kept here so the
/// coverage-measured fuzz binary carries the shape as a seed.
fn dynamic_before_non_first_static_with_live_sibling() -> Vec<Op> {
    vec![
        // span (static, node 1)
        insert_child(
            0,
            0,
            0,
            TemplateNodeKind::Element {
                tag: 1,
                namespace: None,
            },
        ),
        // {a} (dynamic text, node 2) - stays live across both renders
        insert_child(0, 0, 1, TemplateNodeKind::Dynamic(DynamicKind::Text(0))),
        // span (static, node 3)
        insert_child(
            0,
            0,
            2,
            TemplateNodeKind::Element {
                tag: 2,
                namespace: None,
            },
        ),
        // {b} (dynamic, node 4) - starts empty, before a non-first static span
        insert_child(0, 0, 3, TemplateNodeKind::Dynamic(DynamicKind::Empty)),
        // span (static, node 5)
        insert_child(
            0,
            0,
            4,
            TemplateNodeKind::Element {
                tag: 3,
                namespace: None,
            },
        ),
        Op::Rerender,
        // Toggle {b} from Empty -> Text while {a} is already live.
        set_vnode_node(0, 4, TemplateNodeKind::Dynamic(DynamicKind::Text(1))),
        Op::Rerender,
    ]
}

fn root_dynamic_before_static_root_with_nested_dynamic_node() -> Vec<Op> {
    vec![
        set_root_dynamic(),
        insert_element_root(0, 1, 1),
        insert_child(0, 0, 0, TemplateNodeKind::Dynamic(DynamicKind::Text(1))),
        Op::Rerender,
    ]
}

fn root_dynamic_before_static_root_with_nested_dynamic_attr() -> Vec<Op> {
    vec![
        set_root_dynamic(),
        insert_element_root(0, 1, 1),
        insert_child(
            0,
            0,
            0,
            TemplateNodeKind::Element {
                tag: 2,
                namespace: None,
            },
        ),
        insert_dynamic_attr_slot(0, 1, 0),
        insert_dynamic_attr(0, 0, 0, AttrValueSpec::Text(1)),
        Op::Rerender,
    ]
}

fn sibling_dynamic_listener_event() -> Vec<Op> {
    vec![
        insert_child(
            0,
            0,
            0,
            TemplateNodeKind::Element {
                tag: 1,
                namespace: None,
            },
        ),
        insert_dynamic_attr_slot(0, 1, 0),
        insert_child(
            0,
            0,
            1,
            TemplateNodeKind::Element {
                tag: 2,
                namespace: None,
            },
        ),
        insert_dynamic_attr_slot(0, 2, 0),
        insert_dynamic_attr(0, 0, 0, AttrValueSpec::Listener),
        insert_dynamic_attr(0, 1, 0, AttrValueSpec::Listener),
        Op::Rerender,
        Op::fire_event(0, EventBehaviorSpec::Noop),
        Op::fire_event(1, EventBehaviorSpec::Noop),
    ]
}
