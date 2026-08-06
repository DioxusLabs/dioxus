//! Diffing tests for keys that are arbitrary hashable expressions rather than formatted strings.
//!
//! These mirror the scenarios in `diff_keyed_list.rs` and must produce identical mutations.

use dioxus::dioxus_core::{ElementId, Mutation::*};
use dioxus::prelude::*;
use dioxus_core::generation;

/// Should result in moves, but not removals or additions
#[test]
fn hashed_keys_out_of_order() {
    let mut dom = VirtualDom::new(|| {
        let order = match generation() % 2 {
            0 => &[0, 1, 2, 3, /**/ 4, 5, 6, /**/ 7, 8, 9],
            1 => &[0, 1, 2, 3, /**/ 6, 4, 5, /**/ 7, 8, 9],
            _ => unreachable!(),
        };

        rsx!({
            order.iter().map(|i| {
                rsx! {
                    div { key: *i }
                }
            })
        })
    });

    assert_eq!(
        dom.rebuild_to_vec().edits,
        [
            LoadTemplate { index: 0, id: ElementId(1,) },
            LoadTemplate { index: 0, id: ElementId(2,) },
            LoadTemplate { index: 0, id: ElementId(3,) },
            LoadTemplate { index: 0, id: ElementId(4,) },
            LoadTemplate { index: 0, id: ElementId(5,) },
            LoadTemplate { index: 0, id: ElementId(6,) },
            LoadTemplate { index: 0, id: ElementId(7,) },
            LoadTemplate { index: 0, id: ElementId(8,) },
            LoadTemplate { index: 0, id: ElementId(9,) },
            LoadTemplate { index: 0, id: ElementId(10,) },
            AppendChildren { m: 10, id: ElementId(0) },
        ]
    );

    dom.mark_dirty(ScopeId::APP);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [
            PushRoot { id: ElementId(7,) },
            InsertBefore { id: ElementId(5,), m: 1 },
        ]
    );
}

/// A custom type only needs to implement `Hash` to be used as a key
#[test]
fn hashed_keys_custom_type() {
    #[derive(Hash)]
    struct ItemId(u64, u64);

    let mut dom = VirtualDom::new(|| {
        let order: &[u64] = match generation() % 2 {
            0 => &[1, 2, 3],
            1 => &[3, 1, 2],
            _ => unreachable!(),
        };

        rsx!({
            order.iter().map(|i| {
                rsx! {
                    div { key: ItemId(*i, *i + 1) }
                }
            })
        })
    });

    dom.rebuild(&mut dioxus_core::NoOpMutations);

    dom.mark_dirty(ScopeId::APP);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [
            PushRoot { id: ElementId(3,) },
            InsertBefore { id: ElementId(1,), m: 1 },
        ]
    );
}

/// Should result in removals and additions, no shared keys
#[test]
fn no_common_hashed_keys() {
    let mut dom = VirtualDom::new(|| {
        let order: &[_] = match generation() % 2 {
            0 => &[1, 2, 3],
            1 => &[4, 5, 6],
            _ => unreachable!(),
        };

        rsx!({
            order.iter().map(|i| {
                rsx! {
                    div { key: *i }
                }
            })
        })
    });

    dom.rebuild(&mut dioxus_core::NoOpMutations);

    dom.mark_dirty(ScopeId::APP);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [
            LoadTemplate { index: 0, id: ElementId(4) },
            LoadTemplate { index: 0, id: ElementId(5) },
            LoadTemplate { index: 0, id: ElementId(6) },
            Remove { id: ElementId(3) },
            Remove { id: ElementId(2) },
            ReplaceWith { id: ElementId(1), m: 3 }
        ]
    );
}

/// Should result in moves only
#[test]
fn hashed_keys_perfect_reverse() {
    let mut dom = VirtualDom::new(|| {
        let order: &[_] = match generation() % 2 {
            0 => &[1, 2, 3, 4, 5, 6, 7, 8],
            1 => &[9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
            _ => unreachable!(),
        };

        rsx!({
            order.iter().map(|i| {
                rsx! {
                    div { key: *i }
                }
            })
        })
    });

    dom.rebuild(&mut dioxus_core::NoOpMutations);

    dom.mark_dirty(ScopeId::APP);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [
            LoadTemplate { index: 0, id: ElementId(9,) },
            InsertAfter { id: ElementId(1,), m: 1 },
            LoadTemplate { index: 0, id: ElementId(10,) },
            PushRoot { id: ElementId(8,) },
            PushRoot { id: ElementId(7,) },
            PushRoot { id: ElementId(6,) },
            PushRoot { id: ElementId(5,) },
            PushRoot { id: ElementId(4,) },
            PushRoot { id: ElementId(3,) },
            PushRoot { id: ElementId(2,) },
            InsertBefore { id: ElementId(1,), m: 8 },
        ]
    )
}

/// Components can take hashed keys too
#[test]
fn component_hashed_keys() {
    let mut dom = VirtualDom::new(|| {
        let g = generation();

        let order: &[_] = match g % 2 {
            0 => &[0, 1],
            1 => &[1, 0],
            _ => unreachable!(),
        };

        rsx!({
            order.iter().map(|id| {
                rsx! {
                    iter_view { key: *id, id: *id }
                }
            })
        })
    });

    #[component]
    fn iter_view(id: i32) -> Element {
        let text = if id == 0i32 { Some("hey") } else { None };
        rsx! {
            {text}
        }
    }

    assert_eq!(
        dom.rebuild_to_vec().edits,
        [
            CreateTextNode { value: "hey".to_string(), id: ElementId(1,) },
            CreatePlaceholder { id: ElementId(2,) },
            AppendChildren { id: ElementId(0,), m: 2 }
        ]
    );

    dom.mark_dirty(ScopeId::APP);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [
            PushRoot { id: ElementId(2,) },
            InsertBefore { id: ElementId(1,), m: 1 }
        ]
    );
}

/// Hashed keys derived from references and owned values of the same data compare equal
#[test]
fn hashed_keys_stable_across_renders() {
    let mut dom = VirtualDom::new(|| {
        let strings = ["a".to_string(), "b".to_string(), "c".to_string()];

        rsx!({
            strings.iter().map(|s| {
                rsx! {
                    div { key: s }
                }
            })
        })
    });

    dom.rebuild(&mut dioxus_core::NoOpMutations);

    // Rendering the same data again should produce no edits at all
    dom.mark_dirty(ScopeId::APP);
    assert_eq!(dom.render_immediate_to_vec().edits, []);
}
