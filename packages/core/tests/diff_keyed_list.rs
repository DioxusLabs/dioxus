//! Diffing Tests
//!
//! These tests only verify that the diffing algorithm works properly for single components.
//!
//! It does not validate that component lifecycles work properly. This is done in another test file.

use dioxus::prelude::*;
use dioxus_core::generation;
use dioxus_renderer_oracle::{
    EditSummary, OracleNodeId, RendererOracle, SnapshotAttr, SnapshotNode,
};

/// Should result in moves, but not removals or additions
#[test]
fn keyed_diffing_out_of_order() {
    fn app() -> Element {
        let order = match generation() % 2 {
            0 => &[0, 1, 2, 3, /**/ 4, 5, 6, /**/ 7, 8, 9],
            1 => &[0, 1, 2, 3, /**/ 6, 4, 5, /**/ 7, 8, 9],
            _ => unreachable!(),
        };

        rsx!({
            order.iter().map(|i| {
                rsx! {
                    div { key: "{i}", id: "{i}" }
                }
            })
        })
    }

    let (mut dom, mut oracle, _) = rebuild(app, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    let (summary, _) = rerender(&mut dom, &mut oracle, &[0, 1, 2, 3, 6, 4, 5, 7, 8, 9]);
    assert_move_only(summary);
}

/// Should result in moves only
#[test]
fn keyed_diffing_out_of_order_adds() {
    fn app() -> Element {
        let order = match generation() % 2 {
            0 => &[/**/ 4, 5, 6, 7, 8 /**/],
            1 => &[/**/ 8, 7, 4, 5, 6 /**/],
            _ => unreachable!(),
        };

        rsx!({
            order.iter().map(|i| {
                rsx! {
                    div { key: "{i}", id: "{i}" }
                }
            })
        })
    }

    let (mut dom, mut oracle, _) = rebuild(app, &[4, 5, 6, 7, 8]);
    let (summary, _) = rerender(&mut dom, &mut oracle, &[8, 7, 4, 5, 6]);
    assert_move_only(summary);
}

/// Should result in moves only
#[test]
fn keyed_diffing_out_of_order_adds_3() {
    fn app() -> Element {
        let order = match generation() % 2 {
            0 => &[/**/ 4, 5, 6, 7, 8 /**/],
            1 => &[/**/ 4, 8, 7, 5, 6 /**/],
            _ => unreachable!(),
        };

        rsx!({
            order.iter().map(|i| {
                rsx! {
                    div { key: "{i}", id: "{i}" }
                }
            })
        })
    }

    let (mut dom, mut oracle, _) = rebuild(app, &[4, 5, 6, 7, 8]);
    let (summary, _) = rerender(&mut dom, &mut oracle, &[4, 8, 7, 5, 6]);
    assert_move_only(summary);
}

/// Should result in moves only
#[test]
fn keyed_diffing_out_of_order_adds_4() {
    fn app() -> Element {
        let order = match generation() % 2 {
            0 => &[/**/ 4, 5, 6, 7, 8 /**/],
            1 => &[/**/ 4, 5, 8, 7, 6 /**/],
            _ => unreachable!(),
        };

        rsx!({
            order.iter().map(|i| {
                rsx! {
                    div { key: "{i}", id: "{i}" }
                }
            })
        })
    }

    let (mut dom, mut oracle, _) = rebuild(app, &[4, 5, 6, 7, 8]);
    let (summary, _) = rerender(&mut dom, &mut oracle, &[4, 5, 8, 7, 6]);
    assert_move_only(summary);
}

/// Should result in moves only
#[test]
fn keyed_diffing_out_of_order_adds_5() {
    fn app() -> Element {
        let order = match generation() % 2 {
            0 => &[/**/ 4, 5, 6, 7, 8 /**/],
            1 => &[/**/ 4, 5, 6, 8, 7 /**/],
            _ => unreachable!(),
        };

        rsx!({
            order.iter().map(|i| {
                rsx! {
                    div { key: "{i}", id: "{i}" }
                }
            })
        })
    }

    let (mut dom, mut oracle, _) = rebuild(app, &[4, 5, 6, 7, 8]);
    let (summary, _) = rerender(&mut dom, &mut oracle, &[4, 5, 6, 8, 7]);
    assert_move_only(summary);
}

/// Should add the new keyed nodes without recreating existing keyed nodes.
#[test]
fn keyed_diffing_additions() {
    fn app() -> Element {
        let order: &[_] = match generation() % 2 {
            0 => &[/**/ 4, 5, 6, 7, 8 /**/],
            1 => &[/**/ 4, 5, 6, 7, 8, 9, 10 /**/],
            _ => unreachable!(),
        };

        rsx!({
            order.iter().map(|i| {
                rsx! {
                    div { key: "{i}", id: "{i}" }
                }
            })
        })
    }

    let (mut dom, mut oracle, _) = rebuild(app, &[4, 5, 6, 7, 8]);
    let (summary, _) = rerender(&mut dom, &mut oracle, &[4, 5, 6, 7, 8, 9, 10]);
    assert_eq!(summary.loads, 2);
    assert_eq!(summary.removes, 0);
    assert_eq!(summary.replaces, 0);
}

#[test]
fn keyed_diffing_additions_and_moves_on_ends() {
    fn app() -> Element {
        let order: &[_] = match generation() % 2 {
            0 => &[/**/ 4, 5, 6, 7 /**/],
            1 => &[/**/ 7, 4, 5, 6, 11, 12 /**/],
            _ => unreachable!(),
        };

        rsx!({
            order.iter().map(|i| {
                rsx! {
                    div { key: "{i}", id: "{i}" }
                }
            })
        })
    }

    let (mut dom, mut oracle, _) = rebuild(app, &[4, 5, 6, 7]);
    let (summary, _) = rerender(&mut dom, &mut oracle, &[7, 4, 5, 6, 11, 12]);
    assert_eq!(summary.loads, 2);
    assert_eq!(summary.removes, 0);
    assert_eq!(summary.replaces, 0);
}

#[test]
fn keyed_diffing_additions_and_moves_in_middle() {
    fn app() -> Element {
        let order: &[_] = match generation() % 2 {
            0 => &[/**/ 1, 2, 3, 4 /**/],
            1 => &[/**/ 4, 1, 7, 8, 2, 5, 6, 3 /**/],
            _ => unreachable!(),
        };

        rsx!({
            order.iter().map(|i| {
                rsx! {
                    div { key: "{i}", id: "{i}" }
                }
            })
        })
    }

    let (mut dom, mut oracle, _) = rebuild(app, &[1, 2, 3, 4]);
    let (summary, _) = rerender(&mut dom, &mut oracle, &[4, 1, 7, 8, 2, 5, 6, 3]);
    assert_eq!(summary.loads, 4);
    assert_eq!(summary.removes, 0);
    assert_eq!(summary.replaces, 0);
}

#[test]
fn controlled_keyed_diffing_out_of_order() {
    fn app() -> Element {
        let order: &[_] = match generation() % 2 {
            0 => &[4, 5, 6, 7],
            1 => &[0, 5, 9, 6, 4],
            _ => unreachable!(),
        };

        rsx!({
            order.iter().map(|i| {
                rsx! {
                    div { key: "{i}", id: "{i}" }
                }
            })
        })
    }

    let (mut dom, mut oracle, _) = rebuild(app, &[4, 5, 6, 7]);
    let (summary, _) = rerender(&mut dom, &mut oracle, &[0, 5, 9, 6, 4]);
    assert_eq!(summary.loads, 2);
    assert_eq!(summary.removes, 1);
    assert_eq!(summary.replaces, 0);
}

#[test]
fn controlled_keyed_diffing_out_of_order_max_test() {
    fn app() -> Element {
        let order: &[_] = match generation() % 2 {
            0 => &[0, 1, 2, 3, 4],
            1 => &[3, 0, 1, 10, 2],
            _ => unreachable!(),
        };

        rsx!({
            order.iter().map(|i| {
                rsx! {
                    div { key: "{i}", id: "{i}" }
                }
            })
        })
    }

    let (mut dom, mut oracle, _) = rebuild(app, &[0, 1, 2, 3, 4]);
    let (summary, _) = rerender(&mut dom, &mut oracle, &[3, 0, 1, 10, 2]);
    assert_eq!(summary.loads, 1);
    assert_eq!(summary.removes, 1);
    assert_eq!(summary.replaces, 0);
}

// noticed some weird behavior in the desktop interpreter
// just making sure it doesnt happen in the core implementation
#[test]
fn remove_list() {
    fn app() -> Element {
        let order: &[_] = match generation() % 2 {
            0 => &[9, 8, 7, 6, 5],
            1 => &[9, 8],
            _ => unreachable!(),
        };

        rsx!({
            order.iter().map(|i| {
                rsx! {
                    div { key: "{i}", id: "{i}" }
                }
            })
        })
    }

    let (mut dom, mut oracle, _) = rebuild(app, &[9, 8, 7, 6, 5]);
    let (summary, _) = rerender(&mut dom, &mut oracle, &[9, 8]);
    assert_eq!(summary.loads, 0);
    assert_eq!(summary.removes, 3);
    assert_eq!(summary.replaces, 0);
}

#[test]
fn no_common_keys() {
    fn app() -> Element {
        let order: &[_] = match generation() % 2 {
            0 => &[1, 2, 3],
            1 => &[4, 5, 6],
            _ => unreachable!(),
        };

        rsx!({
            order.iter().map(|i| {
                rsx! {
                    div { key: "{i}", id: "{i}" }
                }
            })
        })
    }

    // With no common keys, all 3 old items are removed and 3 new items are loaded.
    let (mut dom, mut oracle, _) = rebuild(app, &[1, 2, 3]);
    let (summary, _) = rerender(&mut dom, &mut oracle, &[4, 5, 6]);
    assert_eq!(summary.loads, 3);
    assert_eq!(summary.removes, 3);
    assert_eq!(summary.replaces, 0);
}

#[test]
fn perfect_reverse() {
    fn app() -> Element {
        let order: &[_] = match generation() % 2 {
            0 => &[1, 2, 3, 4, 5, 6, 7, 8],
            1 => &[9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
            _ => unreachable!(),
        };

        rsx!({
            order.iter().map(|i| {
                rsx! {
                    div { key: "{i}", id: "{i}" }
                }
            })
        })
    }

    let (mut dom, mut oracle, _) = rebuild(app, &[1, 2, 3, 4, 5, 6, 7, 8]);
    let (summary, _) = rerender(&mut dom, &mut oracle, &[9, 8, 7, 6, 5, 4, 3, 2, 1, 0]);
    assert_eq!(summary.loads, 2);
    assert_eq!(summary.removes, 0);
    assert_eq!(summary.replaces, 0);
}

#[test]
fn old_middle_empty_left_pivot() {
    fn app() -> Element {
        let order: &[_] = match generation() % 2 {
            0 => &[/* */ /* */ 6, 7, 8, 9, 10],
            1 => &[/* */ 4, 5, /* */ 6, 7, 8, 9, 10],
            _ => unreachable!(),
        };

        rsx!({
            order.iter().map(|i| {
                rsx! {
                    div { key: "{i}", id: "{i}" }
                }
            })
        })
    }

    let (mut dom, mut oracle, _) = rebuild(app, &[6, 7, 8, 9, 10]);
    let (summary, _) = rerender(&mut dom, &mut oracle, &[4, 5, 6, 7, 8, 9, 10]);
    assert_eq!(summary.loads, 2);
    assert_eq!(summary.removes, 0);
    assert_eq!(summary.replaces, 0);
}

#[test]
fn old_middle_empty_right_pivot() {
    fn app() -> Element {
        let order: &[_] = match generation() % 2 {
            0 => &[1, 2, 3, /*       */ 6, 7, 8, 9, 10],
            1 => &[1, 2, 3, /* */ 4, 5, 6, 7, 8, 9, 10 /* */],

            // 0 => &[/* */ 6, 7, 8, 9, 10],
            // 1 => &[/* */ 6, 7, 8, 9, 10, /* */ 4, 5],
            _ => unreachable!(),
        };

        rsx!({
            order.iter().map(|i| {
                rsx! {
                    div { key: "{i}", id: "{i}" }
                }
            })
        })
    }

    let (mut dom, mut oracle, _) = rebuild(app, &[1, 2, 3, 6, 7, 8, 9, 10]);
    let (summary, _) = rerender(&mut dom, &mut oracle, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    assert_eq!(summary.loads, 2);
    assert_eq!(summary.removes, 0);
    assert_eq!(summary.replaces, 0);
}

/// Regression test for PR #5413
#[test]
fn keyed_list_with_dynamic_placeholder_and_text() {
    fn app() -> Element {
        let g = generation();

        let order: &[_] = match g % 2 {
            0 => &[0, 1],
            1 => &[1, 0],
            _ => unreachable!(),
        };

        rsx!({
            order.iter().map(|id| {
                rsx! {
                    iter_view { key: "{id}", id: *id }
                }
            })
        })
    }

    #[component]
    fn iter_view(id: i32) -> Element {
        let text = if id == 0i32 { Some("hey") } else { None };
        rsx! {
            {text}
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    let rebuild = oracle.rebuild(&mut dom);
    assert_eq!(
        oracle.snapshot(),
        vec![SnapshotNode::Text("hey".to_string())]
    );
    assert_eq!(rebuild.loads, 0);

    dom.mark_dirty(ScopeId::APP);
    let patch = oracle.render(&mut dom);
    assert_eq!(
        oracle.snapshot(),
        vec![SnapshotNode::Text("hey".to_string())]
    );
    assert_eq!(patch, EditSummary::default());
}

fn rebuild(
    app: fn() -> Element,
    expected_order: &[i32],
) -> (VirtualDom, RendererOracle, Vec<(String, OracleNodeId)>) {
    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    assert_keyed_order(&oracle, expected_order);
    let identities = oracle.identities_by_attr("id");
    (dom, oracle, identities)
}

fn rerender(
    dom: &mut VirtualDom,
    oracle: &mut RendererOracle,
    expected_order: &[i32],
) -> (EditSummary, Vec<(String, OracleNodeId)>) {
    let previous = oracle.identities_by_attr("id");
    dom.mark_dirty(ScopeId::APP);
    let summary = oracle.render(dom);
    assert_keyed_order(oracle, expected_order);
    let current = oracle.identities_by_attr("id");
    assert_common_identities_preserved(&previous, &current);
    (summary, current)
}

fn assert_move_only(summary: EditSummary) {
    assert_eq!(summary.loads, 0);
    assert_eq!(summary.removes, 0);
    assert_eq!(summary.replaces, 0);
}

fn assert_keyed_order(oracle: &RendererOracle, expected: &[i32]) {
    assert_eq!(oracle.snapshot(), keyed_divs(expected));
}

fn keyed_divs(ids: &[i32]) -> Vec<SnapshotNode> {
    ids.iter().map(|id| keyed_div(*id)).collect()
}

fn keyed_div(id: i32) -> SnapshotNode {
    SnapshotNode::Element {
        tag: "div".to_string(),
        namespace: None,
        attrs: vec![SnapshotAttr {
            name: "id".to_string(),
            namespace: None,
            value: id.to_string(),
        }],
        listeners: Vec::new(),
        children: Vec::new(),
    }
}

fn assert_common_identities_preserved(
    previous: &[(String, OracleNodeId)],
    current: &[(String, OracleNodeId)],
) {
    for (value, previous_id) in previous {
        if let Some((_, current_id)) = current
            .iter()
            .find(|(current_value, _)| current_value == value)
        {
            assert_eq!(
                previous_id, current_id,
                "node identity for `id={value}` was not preserved"
            );
        }
    }
}
