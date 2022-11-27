#![allow(unused, non_upper_case_globals)]

//! Diffing Tests
//!
//! These tests only verify that the diffing algorithm works properly for single components.
//!
//! It does not validated that component lifecycles work properly. This is done in another test file.

use dioxus::{core_macro::rsx_without_templates, prelude::*};

fn new_dom() -> VirtualDom {
    VirtualDom::new(|cx| cx.render(rsx_without_templates!("hi")))
}

use dioxus_core::DomEdit::*;

/// Should push the text node onto the stack and modify it
#[test]
fn html_and_rsx_generate_the_same_output() {
    let dom = new_dom();
    let (create, change) = dom.diff_lazynodes(
        rsx_without_templates! ( div { "Hello world" } ),
        rsx_without_templates! ( div { "Goodbye world" } ),
    );
    assert_eq!(
        create.edits,
        [
            CreateElement { root: Some(1,), tag: "div", children: 0 },
            CreateTextNode { root: Some(2,), text: "Hello world" },
            AppendChildren { root: Some(1,), children: vec![2] },
            AppendChildren { root: Some(0,), children: vec![1] },
        ]
    );

    assert_eq!(
        change.edits,
        [SetText { root: Some(2,), text: "Goodbye world" },]
    );
}

/// Should result in 3 elements on the stack
#[test]
fn fragments_create_properly() {
    let dom = new_dom();

    let create = dom.create_vnodes(rsx_without_templates! {
        div { "Hello a" }
        div { "Hello b" }
        div { "Hello c" }
    });

    assert_eq!(
        create.edits,
        [
            CreateElement { root: Some(1,), tag: "div", children: 0 },
            CreateTextNode { root: Some(2,), text: "Hello a" },
            AppendChildren { root: Some(1,), children: vec![2,] },
            CreateElement { root: Some(3,), tag: "div", children: 0 },
            CreateTextNode { root: Some(4,), text: "Hello b" },
            AppendChildren { root: Some(3,), children: vec![4,] },
            CreateElement { root: Some(5,), tag: "div", children: 0 },
            CreateTextNode { root: Some(6,), text: "Hello c" },
            AppendChildren { root: Some(5,), children: vec![6,] },
            AppendChildren { root: Some(0,), children: vec![1, 3, 5,] },
        ]
    );
}

/// Should result in the creation of an anchor (placeholder) and then a replacewith
#[test]
fn empty_fragments_create_anchors() {
    let dom = new_dom();

    let left = rsx_without_templates!({ (0..0).map(|_f| rsx_without_templates! { div {}}) });
    let right = rsx_without_templates!({ (0..1).map(|_f| rsx_without_templates! { div {}}) });

    let (create, change) = dom.diff_lazynodes(left, right);

    assert_eq!(
        create.edits,
        [
            CreatePlaceholder { root: Some(1,) },
            AppendChildren { root: Some(0,), children: vec![1,] },
        ]
    );
    assert_eq!(
        change.edits,
        [
            CreateElement { root: Some(2,), tag: "div", children: 0 },
            ReplaceWith { root: Some(1,), nodes: vec![2,] },
        ]
    );
}

/// Should result in the creation of an anchor (placeholder) and then a replacewith m=5
#[test]
fn empty_fragments_create_many_anchors() {
    let dom = new_dom();

    let left = rsx_without_templates!({ (0..0).map(|_f| rsx_without_templates! { div {}}) });
    let right = rsx_without_templates!({ (0..5).map(|_f| rsx_without_templates! { div {}}) });

    let (create, change) = dom.diff_lazynodes(left, right);

    assert_eq!(
        create.edits,
        [
            CreatePlaceholder { root: Some(1,) },
            AppendChildren { root: Some(0,), children: vec![1,] },
        ]
    );

    assert_eq!(
        change.edits,
        [
            CreateElement { root: Some(2,), tag: "div", children: 0 },
            CreateElement { root: Some(3,), tag: "div", children: 0 },
            CreateElement { root: Some(4,), tag: "div", children: 0 },
            CreateElement { root: Some(5,), tag: "div", children: 0 },
            CreateElement { root: Some(6,), tag: "div", children: 0 },
            ReplaceWith { root: Some(1,), nodes: vec![2, 3, 4, 5, 6,] },
        ]
    );
}

/// Should result in the creation of an anchor (placeholder) and then a replacewith
/// Includes child nodes inside the fragment
#[test]
fn empty_fragments_create_anchors_with_many_children() {
    let dom = new_dom();

    let left = rsx_without_templates!({ (0..0).map(|_| rsx_without_templates! { div {} }) });
    let right = rsx_without_templates!({
        (0..3).map(|f| {
            rsx_without_templates! { div { "hello: {f}" }}
        })
    });

    let (create, change) = dom.diff_lazynodes(left, right);

    assert_eq!(
        create.edits,
        [
            CreatePlaceholder { root: Some(1,) },
            AppendChildren { root: Some(0,), children: vec![1,] },
        ]
    );

    assert_eq!(
        change.edits,
        [
            CreateElement { root: Some(2,), tag: "div", children: 0 },
            CreateTextNode { root: Some(3,), text: "hello: 0" },
            AppendChildren { root: Some(2,), children: vec![3,] },
            CreateElement { root: Some(4,), tag: "div", children: 0 },
            CreateTextNode { root: Some(5,), text: "hello: 1" },
            AppendChildren { root: Some(4,), children: vec![5,] },
            CreateElement { root: Some(6,), tag: "div", children: 0 },
            CreateTextNode { root: Some(7,), text: "hello: 2" },
            AppendChildren { root: Some(6,), children: vec![7,] },
            ReplaceWith { root: Some(1,), nodes: vec![2, 4, 6,] },
        ]
    );
}

/// Should result in every node being pushed and then replaced with an anchor
#[test]
fn many_items_become_fragment() {
    let dom = new_dom();

    let left = rsx_without_templates!({
        (0..2).map(|_| {
            rsx_without_templates! { div { "hello" }}
        })
    });
    let right = rsx_without_templates!({ (0..0).map(|_| rsx_without_templates! { div {} }) });

    let (create, change) = dom.diff_lazynodes(left, right);

    assert_eq!(
        create.edits,
        [
            CreateElement { root: Some(1,), tag: "div", children: 0 },
            CreateTextNode { root: Some(2,), text: "hello" },
            AppendChildren { root: Some(1,), children: vec![2,] },
            CreateElement { root: Some(3,), tag: "div", children: 0 },
            CreateTextNode { root: Some(4,), text: "hello" },
            AppendChildren { root: Some(3,), children: vec![4,] },
            AppendChildren { root: Some(0,), children: vec![1, 3,] },
        ]
    );

    assert_eq!(
        change.edits,
        [
            CreatePlaceholder { root: Some(5,) },
            ReplaceWith { root: Some(1,), nodes: vec![5,] },
            Remove { root: Some(3,) },
        ]
    );
}

/// Should result in no edits
#[test]
fn two_equal_fragments_are_equal() {
    let dom = new_dom();

    let left = rsx_without_templates!({
        (0..2).map(|_| {
            rsx_without_templates! { div { "hello" }}
        })
    });
    let right = rsx_without_templates!({
        (0..2).map(|_| {
            rsx_without_templates! { div { "hello" }}
        })
    });

    let (_create, change) = dom.diff_lazynodes(left, right);
    assert!(change.edits.is_empty());
}

/// Should result the creation of more nodes appended after the old last node
#[test]
fn two_fragments_with_differrent_elements_are_differet() {
    let dom = new_dom();

    let left = rsx_without_templates!(
        { (0..2).map(|_| rsx_without_templates! { div {  }} ) }
        p {}
    );
    let right = rsx_without_templates!(
        { (0..5).map(|_| rsx_without_templates! (h1 {  }) ) }
        p {}
    );

    let (_create, changes) = dom.diff_lazynodes(left, right);
    assert_eq!(
        changes.edits,
        [
            CreateElement { root: Some(4,), tag: "h1", children: 0 },
            CreateElement { root: Some(5,), tag: "h1", children: 0 },
            CreateElement { root: Some(6,), tag: "h1", children: 0 },
            InsertAfter { root: Some(2,), nodes: vec![4, 5, 6,] },
            CreateElement { root: Some(7,), tag: "h1", children: 0 },
            ReplaceWith { root: Some(1,), nodes: vec![7,] }, // notice how 1 gets re-used
            CreateElement { root: Some(1,), tag: "h1", children: 0 },
            ReplaceWith { root: Some(2,), nodes: vec![1,] },
        ]
    );
}

/// Should result in multiple nodes destroyed - with changes to the first nodes
#[test]
fn two_fragments_with_differrent_elements_are_differet_shorter() {
    let dom = new_dom();

    let left = rsx_without_templates!(
        {(0..5).map(|f| {rsx_without_templates! { div {  }}})}
        p {}
    );
    let right = rsx_without_templates!(
        {(0..2).map(|f| {rsx_without_templates! { h1 {  }}})}
        p {}
    );

    let (create, change) = dom.diff_lazynodes(left, right);

    assert_eq!(
        create.edits,
        [
            CreateElement { root: Some(1,), tag: "div", children: 0 },
            CreateElement { root: Some(2,), tag: "div", children: 0 },
            CreateElement { root: Some(3,), tag: "div", children: 0 },
            CreateElement { root: Some(4,), tag: "div", children: 0 },
            CreateElement { root: Some(5,), tag: "div", children: 0 },
            CreateElement { root: Some(6,), tag: "p", children: 0 },
            AppendChildren { root: Some(0,), children: vec![1, 2, 3, 4, 5, 6,] },
        ]
    );

    // note: key reuse is always the last node that got used
    // slab maintains a linked list, essentially
    assert_eq!(
        change.edits,
        [
            Remove { root: Some(3,) },
            Remove { root: Some(4,) },
            Remove { root: Some(5,) },
            CreateElement { root: Some(5,), tag: "h1", children: 0 }, // 5 gets reused
            ReplaceWith { root: Some(1,), nodes: vec![5,] },          // 1 gets deleted
            CreateElement { root: Some(1,), tag: "h1", children: 0 }, // 1 gets reused
            ReplaceWith { root: Some(2,), nodes: vec![1,] },
        ]
    );
}

/// Should result in multiple nodes destroyed - with no changes
#[test]
fn two_fragments_with_same_elements_are_differet() {
    let dom = new_dom();

    let left = rsx_without_templates!(
        {(0..2).map(|f| rsx_without_templates! { div {  }})}
        p {}
    );
    let right = rsx_without_templates!(
        {(0..5).map(|f| rsx_without_templates! { div {  }})}
        p {}
    );

    let (create, change) = dom.diff_lazynodes(left, right);

    assert_eq!(
        create.edits,
        [
            CreateElement { root: Some(1,), tag: "div", children: 0 },
            CreateElement { root: Some(2,), tag: "div", children: 0 },
            CreateElement { root: Some(3,), tag: "p", children: 0 },
            AppendChildren { root: Some(0,), children: vec![1, 2, 3,] },
        ]
    );
    assert_eq!(
        change.edits,
        [
            CreateElement { root: Some(4,), tag: "div", children: 0 },
            CreateElement { root: Some(5,), tag: "div", children: 0 },
            CreateElement { root: Some(6,), tag: "div", children: 0 },
            InsertAfter { root: Some(2,), nodes: vec![4, 5, 6,] },
        ]
    );
}

/// should result in the removal of elements
#[test]
fn keyed_diffing_order() {
    let dom = new_dom();

    let left = rsx_without_templates!(
        {(0..5).map(|f| {rsx_without_templates! { div { key: "{f}"  }}})}
        p {"e"}
    );
    let right = rsx_without_templates!(
        {(0..2).map(|f| rsx_without_templates! { div { key: "{f}" }})}
        p {"e"}
    );

    let (create, change) = dom.diff_lazynodes(left, right);

    assert_eq!(
        change.edits,
        [
            Remove { root: Some(3,) },
            Remove { root: Some(4,) },
            Remove { root: Some(5,) },
        ]
    );
}

/// Should result in moves, but not removals or additions
#[test]
fn keyed_diffing_out_of_order() {
    let dom = new_dom();

    let left = rsx_without_templates!({
        [0, 1, 2, 3, /**/ 4, 5, 6, /**/ 7, 8, 9].iter().map(|f| {
            rsx_without_templates! { div { key: "{f}"  }}
        })
    });

    let right = rsx_without_templates!({
        [0, 1, 2, 3, /**/ 6, 4, 5, /**/ 7, 8, 9].iter().map(|f| {
            rsx_without_templates! { div { key: "{f}"  }}
        })
    });

    let (_, changes) = dom.diff_lazynodes(left, right);

    assert_eq!(
        changes.edits,
        [InsertBefore { root: Some(5,), nodes: vec![7,] },]
    );
}

/// Should result in moves only
#[test]
fn keyed_diffing_out_of_order_adds() {
    let dom = new_dom();

    let left = rsx_without_templates!({
        [/**/ 4, 5, 6, 7, 8 /**/].iter().map(|f| {
            rsx_without_templates! { div { key: "{f}"  }}
        })
    });

    let right = rsx_without_templates!({
        [/**/ 8, 7, 4, 5, 6 /**/].iter().map(|f| {
            rsx_without_templates! { div { key: "{f}"  }}
        })
    });

    let (_, change) = dom.diff_lazynodes(left, right);

    assert_eq!(
        change.edits,
        [InsertBefore { root: Some(1,), nodes: vec![5, 4,] },]
    );
}
/// Should result in moves only
#[test]
fn keyed_diffing_out_of_order_adds_2() {
    let dom = new_dom();

    let left = rsx_without_templates!({
        [/**/ 4, 5, 6, 7, 8 /**/].iter().map(|f| {
            rsx_without_templates! { div { key: "{f}"  }}
        })
    });

    let right = rsx_without_templates!({
        [/**/ 7, 8, 4, 5, 6 /**/].iter().map(|f| {
            rsx_without_templates! { div { key: "{f}"  }}
        })
    });

    let (_, change) = dom.diff_lazynodes(left, right);

    assert_eq!(
        change.edits,
        [InsertBefore { root: Some(1,), nodes: vec![4, 5,] },]
    );
}

/// Should result in moves onl
#[test]
fn keyed_diffing_out_of_order_adds_3() {
    let dom = new_dom();

    let left = rsx_without_templates!({
        [/**/ 4, 5, 6, 7, 8 /**/].iter().map(|f| {
            rsx_without_templates! { div { key: "{f}"  }}
        })
    });

    let right = rsx_without_templates!({
        [/**/ 4, 8, 7, 5, 6 /**/].iter().map(|f| {
            rsx_without_templates! { div { key: "{f}"  }}
        })
    });

    let (_, change) = dom.diff_lazynodes(left, right);

    assert_eq!(
        change.edits,
        [InsertBefore { root: Some(2,), nodes: vec![5, 4,] },]
    );
}

/// Should result in moves onl
#[test]
fn keyed_diffing_out_of_order_adds_4() {
    let dom = new_dom();

    let left = rsx_without_templates!({
        [/**/ 4, 5, 6, 7, 8 /**/].iter().map(|f| {
            rsx_without_templates! { div { key: "{f}"  }}
        })
    });

    let right = rsx_without_templates!({
        [/**/ 4, 5, 8, 7, 6 /**/].iter().map(|f| {
            rsx_without_templates! { div { key: "{f}"  }}
        })
    });

    let (_, change) = dom.diff_lazynodes(left, right);

    assert_eq!(
        change.edits,
        [InsertBefore { root: Some(3), nodes: vec![5, 4,] },]
    );
}

/// Should result in moves onl
#[test]
fn keyed_diffing_out_of_order_adds_5() {
    let dom = new_dom();

    let left = rsx_without_templates!({
        [/**/ 4, 5, 6, 7, 8 /**/].iter().map(|f| {
            rsx_without_templates! { div { key: "{f}"  }}
        })
    });

    let right = rsx_without_templates!({
        [/**/ 4, 5, 6, 8, 7 /**/].iter().map(|f| {
            rsx_without_templates! { div { key: "{f}"  }}
        })
    });

    let (_, change) = dom.diff_lazynodes(left, right);
    assert_eq!(
        change.edits,
        [InsertBefore { root: Some(4), nodes: vec![5] }]
    );
}

#[test]
fn keyed_diffing_additions() {
    let dom = new_dom();

    let left = rsx_without_templates!({
        [/**/ 4, 5, 6, 7, 8 /**/].iter().map(|f| {
            rsx_without_templates! { div { key: "{f}"  }}
        })
    });

    let right = rsx_without_templates!({
        [/**/ 4, 5, 6, 7, 8, 9, 10 /**/].iter().map(|f| {
            rsx_without_templates! { div { key: "{f}"  }}
        })
    });

    let (_, change) = dom.diff_lazynodes(left, right);

    assert_eq!(
        change.edits,
        [
            CreateElement { root: Some(6,), tag: "div", children: 0 },
            CreateElement { root: Some(7,), tag: "div", children: 0 },
            InsertAfter { root: Some(5,), nodes: vec![6, 7,] },
        ]
    );
}

#[test]
fn keyed_diffing_additions_and_moves_on_ends() {
    let dom = new_dom();

    let left = rsx_without_templates!({
        [/**/ 4, 5, 6, 7 /**/].iter().map(|f| {
            rsx_without_templates! { div { key: "{f}"  }}
        })
    });

    let right = rsx_without_templates!({
        [/**/ 7, 4, 5, 6, 11, 12 /**/].iter().map(|f| {
            rsx_without_templates! { div { key: "{f}"  }}
        })
    });

    let (_, change) = dom.diff_lazynodes(left, right);
    println!("{:?}", change);
    assert_eq!(
        change.edits,
        [
            // create 11, 12
            CreateElement { root: Some(5), tag: "div", children: 0 },
            CreateElement { root: Some(6), tag: "div", children: 0 },
            InsertAfter { root: Some(3), nodes: vec![5, 6] },
            // // move 7 to the front
            InsertBefore { root: Some(1), nodes: vec![4] }
        ]
    );
}

#[test]
fn keyed_diffing_additions_and_moves_in_middle() {
    let dom = new_dom();

    let left = rsx_without_templates!({
        [/**/ 1, 2, 3, 4 /**/].iter().map(|f| {
            rsx_without_templates! { div { key: "{f}"  }}
        })
    });

    let right = rsx_without_templates!({
        [/**/ 4, 1, 7, 8, 2, 5, 6, 3 /**/].iter().map(|f| {
            rsx_without_templates! { div { key: "{f}"  }}
        })
    });

    // LIS: 4, 5, 6
    let (_, change) = dom.diff_lazynodes(left, right);
    println!("{:#?}", change);
    assert_eq!(
        change.edits,
        [
            // create 5, 6
            CreateElement { root: Some(5,), tag: "div", children: 0 },
            CreateElement { root: Some(6,), tag: "div", children: 0 },
            InsertBefore { root: Some(3,), nodes: vec![5, 6,] },
            // create 7, 8
            CreateElement { root: Some(7,), tag: "div", children: 0 },
            CreateElement { root: Some(8,), tag: "div", children: 0 },
            InsertBefore { root: Some(2,), nodes: vec![7, 8,] },
            // move 7
            InsertBefore { root: Some(1,), nodes: vec![4,] },
        ]
    );
}

#[test]
fn controlled_keyed_diffing_out_of_order() {
    let dom = new_dom();

    let left = rsx_without_templates!({
        [4, 5, 6, 7].iter().map(|f| {
            rsx_without_templates! { div { key: "{f}" }}
        })
    });

    let right = rsx_without_templates!({
        [0, 5, 9, 6, 4].iter().map(|f| {
            rsx_without_templates! { div { key: "{f}" }}
        })
    });

    // LIS: 5, 6
    let (_, changes) = dom.diff_lazynodes(left, right);
    println!("{:#?}", &changes);
    assert_eq!(
        changes.edits,
        [
            // remove 7
            Remove { root: Some(4,) },
            // move 4 to after 6
            InsertAfter { root: Some(3,), nodes: vec![1,] },
            // create 9 and insert before 6
            CreateElement { root: Some(4,), tag: "div", children: 0 },
            InsertBefore { root: Some(3,), nodes: vec![4,] },
            // create 0 and insert before 5
            CreateElement { root: Some(5,), tag: "div", children: 0 },
            InsertBefore { root: Some(2,), nodes: vec![5,] },
        ]
    );
}

#[test]
fn controlled_keyed_diffing_out_of_order_max_test() {
    let dom = new_dom();

    let left = rsx_without_templates!({
        [0, 1, 2, 3, 4].iter().map(|f| {
            rsx_without_templates! { div { key: "{f}"  }}
        })
    });

    let right = rsx_without_templates!({
        [3, 0, 1, 10, 2].iter().map(|f| {
            rsx_without_templates! { div { key: "{f}"  }}
        })
    });

    let (_, changes) = dom.diff_lazynodes(left, right);
    println!("{:#?}", &changes);
    assert_eq!(
        changes.edits,
        [
            Remove { root: Some(5,) },
            CreateElement { root: Some(5,), tag: "div", children: 0 },
            InsertBefore { root: Some(3,), nodes: vec![5,] },
            InsertBefore { root: Some(1,), nodes: vec![4,] },
        ]
    );
}

// noticed some weird behavior in the desktop interpreter
// just making sure it doesnt happen in the core implementation
#[test]
fn remove_list() {
    let dom = new_dom();

    let left = rsx_without_templates!({
        (0..10).rev().take(5).map(|i| {
            rsx_without_templates! { Fragment { key: "{i}", "{i}" }}
        })
    });

    let right = rsx_without_templates!({
        (0..10).rev().take(2).map(|i| {
            rsx_without_templates! { Fragment { key: "{i}", "{i}" }}
        })
    });

    let (create, changes) = dom.diff_lazynodes(left, right);

    // dbg!(create);
    // dbg!(changes);

    assert_eq!(
        changes.edits,
        // remove 5, 4, 3
        [
            Remove { root: Some(3) },
            Remove { root: Some(4) },
            Remove { root: Some(5) }
        ]
    );
}

// noticed some weird behavior in the desktop interpreter
// just making sure it doesnt happen in the core implementation
#[test]
fn remove_list_nokeyed() {
    let dom = new_dom();

    let left = rsx_without_templates!({
        (0..10).rev().take(5).map(|i| {
            rsx_without_templates! { Fragment { "{i}" }}
        })
    });

    let right = rsx_without_templates!({
        (0..10).rev().take(2).map(|i| {
            rsx_without_templates! { Fragment { "{i}" }}
        })
    });

    let (create, changes) = dom.diff_lazynodes(left, right);

    assert_eq!(
        changes.edits,
        [
            // remove 5, 4, 3
            Remove { root: Some(3) },
            Remove { root: Some(4) },
            Remove { root: Some(5) },
        ]
    );
}

#[test]
fn add_nested_elements() {
    let vdom = new_dom();

    let (_create, change) = vdom.diff_lazynodes(
        rsx_without_templates! {
            div{}
        },
        rsx_without_templates! {
            div{
                div{}
            }
        },
    );

    assert_eq!(
        change.edits,
        [
            CreateElement { root: Some(2), tag: "div", children: 0 },
            AppendChildren { root: Some(1), children: vec![2] },
        ]
    );
}

#[test]
fn add_listeners() {
    let vdom = new_dom();

    let (_create, change) = vdom.diff_lazynodes(
        rsx_without_templates! {
            div{}
        },
        rsx_without_templates! {
            div{
                onkeyup: |_| {},
                onkeydown: |_| {},
            }
        },
    );

    assert_eq!(
        change.edits,
        [
            NewEventListener { event_name: "keyup", scope: ScopeId(0), root: Some(1) },
            NewEventListener { event_name: "keydown", scope: ScopeId(0), root: Some(1) },
        ]
    );
}

#[test]
fn remove_listeners() {
    let vdom = new_dom();

    let (_create, change) = vdom.diff_lazynodes(
        rsx_without_templates! {
            div{
                onkeyup: |_| {},
                onkeydown: |_| {},
            }
        },
        rsx_without_templates! {
            div{}
        },
    );

    assert_eq!(
        change.edits,
        [
            RemoveEventListener { event: "keyup", root: Some(1) },
            RemoveEventListener { event: "keydown", root: Some(1) },
        ]
    );
}

#[test]
fn diff_listeners() {
    let vdom = new_dom();

    let (_create, change) = vdom.diff_lazynodes(
        rsx_without_templates! {
            div{
                onkeydown: |_| {},
            }
        },
        rsx_without_templates! {
            div{
                onkeyup: |_| {},
            }
        },
    );

    assert_eq!(
        change.edits,
        [
            RemoveEventListener { root: Some(1), event: "keydown" },
            NewEventListener { event_name: "keyup", scope: ScopeId(0), root: Some(1) }
        ]
    );
}
