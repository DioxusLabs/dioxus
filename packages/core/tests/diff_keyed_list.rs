//! Diffing Tests
//!
//! These tests only verify that the diffing algorithm works properly for single components.
//!
//! It does not validated that component lifecycles work properly. This is done in another test file.

use dioxus::dioxus_core::{ElementId, Mutation::*};
use dioxus::prelude::*;

/// Should result in moves, but not removals or additions
#[test]
fn keyed_diffing_out_of_order() {
    let mut dom = VirtualDom::new(|| {
        let order = match generation() % 2 {
            0 => &[0, 1, 2, 3, /**/ 4, 5, 6, /**/ 7, 8, 9],
            1 => &[0, 1, 2, 3, /**/ 6, 4, 5, /**/ 7, 8, 9],
            _ => unreachable!(),
        };

        rsx!({ order.iter().map(|i| rsx!(div { key: "{i}" })) })
    });

    {
        assert_eq!(
            dom.rebuild_to_vec().santize().edits,
            [
                LoadTemplate { name: "template", index: 0, id: ElementId(1,) },
                LoadTemplate { name: "template", index: 0, id: ElementId(2,) },
                LoadTemplate { name: "template", index: 0, id: ElementId(3,) },
                LoadTemplate { name: "template", index: 0, id: ElementId(4,) },
                LoadTemplate { name: "template", index: 0, id: ElementId(5,) },
                LoadTemplate { name: "template", index: 0, id: ElementId(6,) },
                LoadTemplate { name: "template", index: 0, id: ElementId(7,) },
                LoadTemplate { name: "template", index: 0, id: ElementId(8,) },
                LoadTemplate { name: "template", index: 0, id: ElementId(9,) },
                LoadTemplate { name: "template", index: 0, id: ElementId(10,) },
                AppendChildren { m: 10, id: ElementId(0) },
            ]
        );
    }

    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [
            PushRoot { id: ElementId(7,) },
            InsertBefore { id: ElementId(5,), m: 1 },
        ]
    );
}

/// Should result in moves only
#[test]
fn keyed_diffing_out_of_order_adds() {
    let mut dom = VirtualDom::new(|| {
        let order = match generation() % 2 {
            0 => &[/**/ 4, 5, 6, 7, 8 /**/],
            1 => &[/**/ 8, 7, 4, 5, 6 /**/],
            _ => unreachable!(),
        };

        rsx!({ order.iter().map(|i| rsx!(div { key: "{i}" })) })
    });

    dom.rebuild(&mut dioxus_core::NoOpMutations);

    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [
            PushRoot { id: ElementId(5,) },
            PushRoot { id: ElementId(4,) },
            InsertBefore { id: ElementId(1,), m: 2 },
        ]
    );
}

/// Should result in moves only
#[test]
fn keyed_diffing_out_of_order_adds_3() {
    let mut dom = VirtualDom::new(|| {
        let order = match generation() % 2 {
            0 => &[/**/ 4, 5, 6, 7, 8 /**/],
            1 => &[/**/ 4, 8, 7, 5, 6 /**/],
            _ => unreachable!(),
        };

        rsx!({ order.iter().map(|i| rsx!(div { key: "{i}" })) })
    });

    dom.rebuild(&mut dioxus_core::NoOpMutations);

    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [
            PushRoot { id: ElementId(5,) },
            PushRoot { id: ElementId(4,) },
            InsertBefore { id: ElementId(2,), m: 2 },
        ]
    );
}

/// Should result in moves onl
#[test]
fn keyed_diffing_out_of_order_adds_4() {
    let mut dom = VirtualDom::new(|| {
        let order = match generation() % 2 {
            0 => &[/**/ 4, 5, 6, 7, 8 /**/],
            1 => &[/**/ 4, 5, 8, 7, 6 /**/],
            _ => unreachable!(),
        };

        rsx!({ order.iter().map(|i| rsx!(div { key: "{i}" })) })
    });

    dom.rebuild(&mut dioxus_core::NoOpMutations);

    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [
            PushRoot { id: ElementId(5,) },
            PushRoot { id: ElementId(4,) },
            InsertBefore { id: ElementId(3,), m: 2 },
        ]
    );
}

/// Should result in moves onl
#[test]
fn keyed_diffing_out_of_order_adds_5() {
    let mut dom = VirtualDom::new(|| {
        let order = match generation() % 2 {
            0 => &[/**/ 4, 5, 6, 7, 8 /**/],
            1 => &[/**/ 4, 5, 6, 8, 7 /**/],
            _ => unreachable!(),
        };

        rsx!({ order.iter().map(|i| rsx!(div { key: "{i}" })) })
    });

    dom.rebuild(&mut dioxus_core::NoOpMutations);

    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [
            PushRoot { id: ElementId(5,) },
            InsertBefore { id: ElementId(4,), m: 1 },
        ]
    );
}

/// Should result in moves onl
#[test]
fn keyed_diffing_additions() {
    let mut dom = VirtualDom::new(|| {
        let order: &[_] = match generation() % 2 {
            0 => &[/**/ 4, 5, 6, 7, 8 /**/],
            1 => &[/**/ 4, 5, 6, 7, 8, 9, 10 /**/],
            _ => unreachable!(),
        };

        rsx!({ order.iter().map(|i| rsx!(div { key: "{i}" })) })
    });

    dom.rebuild(&mut dioxus_core::NoOpMutations);

    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(6) },
            LoadTemplate { name: "template", index: 0, id: ElementId(7) },
            InsertAfter { id: ElementId(5), m: 2 }
        ]
    );
}

#[test]
fn keyed_diffing_additions_and_moves_on_ends() {
    let mut dom = VirtualDom::new(|| {
        let order: &[_] = match generation() % 2 {
            0 => &[/**/ 4, 5, 6, 7 /**/],
            1 => &[/**/ 7, 4, 5, 6, 11, 12 /**/],
            _ => unreachable!(),
        };

        rsx!({ order.iter().map(|i| rsx!(div { key: "{i}" })) })
    });

    dom.rebuild(&mut dioxus_core::NoOpMutations);

    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
        [
            // create 11, 12
            LoadTemplate { name: "template", index: 0, id: ElementId(5) },
            LoadTemplate { name: "template", index: 0, id: ElementId(6) },
            InsertAfter { id: ElementId(3), m: 2 },
            // move 7 to the front
            PushRoot { id: ElementId(4) },
            InsertBefore { id: ElementId(1), m: 1 }
        ]
    );
}

#[test]
fn keyed_diffing_additions_and_moves_in_middle() {
    let mut dom = VirtualDom::new(|| {
        let order: &[_] = match generation() % 2 {
            0 => &[/**/ 1, 2, 3, 4 /**/],
            1 => &[/**/ 4, 1, 7, 8, 2, 5, 6, 3 /**/],
            _ => unreachable!(),
        };

        rsx!({ order.iter().map(|i| rsx!(div { key: "{i}" })) })
    });

    dom.rebuild(&mut dioxus_core::NoOpMutations);

    // LIS: 4, 5, 6
    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
        [
            // create 5, 6
            LoadTemplate { name: "template", index: 0, id: ElementId(5) },
            LoadTemplate { name: "template", index: 0, id: ElementId(6) },
            InsertBefore { id: ElementId(3), m: 2 },
            // create 7, 8
            LoadTemplate { name: "template", index: 0, id: ElementId(7) },
            LoadTemplate { name: "template", index: 0, id: ElementId(8) },
            InsertBefore { id: ElementId(2), m: 2 },
            // move 7
            PushRoot { id: ElementId(4) },
            InsertBefore { id: ElementId(1), m: 1 }
        ]
    );
}

#[test]
fn controlled_keyed_diffing_out_of_order() {
    let mut dom = VirtualDom::new(|| {
        let order: &[_] = match generation() % 2 {
            0 => &[4, 5, 6, 7],
            1 => &[0, 5, 9, 6, 4],
            _ => unreachable!(),
        };

        rsx!({ order.iter().map(|i| rsx!(div { key: "{i}" })) })
    });

    dom.rebuild(&mut dioxus_core::NoOpMutations);

    // LIS: 5, 6
    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
        [
            // remove 7
            Remove { id: ElementId(4,) },
            // move 4 to after 6
            PushRoot { id: ElementId(1) },
            InsertAfter { id: ElementId(3,), m: 1 },
            // create 9 and insert before 6
            LoadTemplate { name: "template", index: 0, id: ElementId(4) },
            InsertBefore { id: ElementId(3,), m: 1 },
            // create 0 and insert before 5
            LoadTemplate { name: "template", index: 0, id: ElementId(5) },
            InsertBefore { id: ElementId(2,), m: 1 },
        ]
    );
}

#[test]
fn controlled_keyed_diffing_out_of_order_max_test() {
    let mut dom = VirtualDom::new(|| {
        let order: &[_] = match generation() % 2 {
            0 => &[0, 1, 2, 3, 4],
            1 => &[3, 0, 1, 10, 2],
            _ => unreachable!(),
        };

        rsx!({ order.iter().map(|i| rsx!(div { key: "{i}" })) })
    });

    dom.rebuild(&mut dioxus_core::NoOpMutations);

    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
        [
            Remove { id: ElementId(5,) },
            LoadTemplate { name: "template", index: 0, id: ElementId(5) },
            InsertBefore { id: ElementId(3,), m: 1 },
            PushRoot { id: ElementId(4) },
            InsertBefore { id: ElementId(1,), m: 1 },
        ]
    );
}

// noticed some weird behavior in the desktop interpreter
// just making sure it doesnt happen in the core implementation
#[test]
fn remove_list() {
    let mut dom = VirtualDom::new(|| {
        let order: &[_] = match generation() % 2 {
            0 => &[9, 8, 7, 6, 5],
            1 => &[9, 8],
            _ => unreachable!(),
        };

        rsx!({ order.iter().map(|i| rsx!(div { key: "{i}" })) })
    });

    dom.rebuild(&mut dioxus_core::NoOpMutations);

    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
        [
            Remove { id: ElementId(5) },
            Remove { id: ElementId(4) },
            Remove { id: ElementId(3) },
        ]
    );
}

#[test]
fn no_common_keys() {
    let mut dom = VirtualDom::new(|| {
        let order: &[_] = match generation() % 2 {
            0 => &[1, 2, 3],
            1 => &[4, 5, 6],
            _ => unreachable!(),
        };

        rsx!({ order.iter().map(|i| rsx!(div { key: "{i}" })) })
    });

    dom.rebuild(&mut dioxus_core::NoOpMutations);

    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(4) },
            LoadTemplate { name: "template", index: 0, id: ElementId(5) },
            LoadTemplate { name: "template", index: 0, id: ElementId(6) },
            Remove { id: ElementId(3) },
            Remove { id: ElementId(2) },
            ReplaceWith { id: ElementId(1), m: 3 }
        ]
    );
}
