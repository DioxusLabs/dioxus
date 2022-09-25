#![allow(unused, non_upper_case_globals)]

//! Diffing Tests
//!
//! These tests only verify that the diffing algorithm works properly for single components.
//!
//! It does not validated that component lifecycles work properly. This is done in another test file.

use dioxus::prelude::*;

fn new_dom() -> VirtualDom {
    VirtualDom::new(|cx| render!("hi"))
}

use dioxus_core::DomEdit::*;

/// Should push the text node onto the stack and modify it
#[test]
fn html_and_rsx_generate_the_same_output() {
    let dom = new_dom();
    let (create, change) = dom.diff_lazynodes(
        rsx! ( div { "Hello world" } ),
        rsx! ( div { "Goodbye world" } ),
    );
    assert_eq!(
        create.edits,
        [
            CreateElement { root: 1, tag: "div" },
            CreateTextNode { root: 2, text: "Hello world" },
            AppendChildren { many: 1 },
            AppendChildren { many: 1 },
        ]
    );

    assert_eq!(change.edits, [SetText { text: "Goodbye world", root: 2 },]);
}

/// Should result in 3 elements on the stack
#[test]
fn fragments_create_properly() {
    let dom = new_dom();

    let create = dom.create_vnodes(rsx! {
        div { "Hello a" }
        div { "Hello b" }
        div { "Hello c" }
    });

    assert_eq!(
        create.edits,
        [
            CreateElement { root: 1, tag: "div" },
            CreateTextNode { root: 2, text: "Hello a" },
            AppendChildren { many: 1 },
            CreateElement { root: 3, tag: "div" },
            CreateTextNode { root: 4, text: "Hello b" },
            AppendChildren { many: 1 },
            CreateElement { root: 5, tag: "div" },
            CreateTextNode { root: 6, text: "Hello c" },
            AppendChildren { many: 1 },
            AppendChildren { many: 3 },
        ]
    );
}

/// Should result in the creation of an anchor (placeholder) and then a replacewith
#[test]
fn empty_fragments_create_anchors() {
    let dom = new_dom();

    let left = rsx!({ (0..0).map(|_f| rsx! { div {}}) });
    let right = rsx!({ (0..1).map(|_f| rsx! { div {}}) });

    let (create, change) = dom.diff_lazynodes(left, right);

    assert_eq!(
        create.edits,
        [CreatePlaceholder { root: 1 }, AppendChildren { many: 1 }]
    );
    assert_eq!(
        change.edits,
        [
            CreateElement { root: 2, tag: "div" },
            ReplaceWith { m: 1, root: 1 }
        ]
    );
}

/// Should result in the creation of an anchor (placeholder) and then a replacewith m=5
#[test]
fn empty_fragments_create_many_anchors() {
    let dom = new_dom();

    let left = rsx!({ (0..0).map(|_f| rsx! { div {}}) });
    let right = rsx!({ (0..5).map(|_f| rsx! { div {}}) });

    let (create, change) = dom.diff_lazynodes(left, right);
    assert_eq!(
        create.edits,
        [CreatePlaceholder { root: 1 }, AppendChildren { many: 1 }]
    );

    assert_eq!(
        change.edits,
        [
            CreateElement { root: 2, tag: "div" },
            CreateElement { root: 3, tag: "div" },
            CreateElement { root: 4, tag: "div" },
            CreateElement { root: 5, tag: "div" },
            CreateElement { root: 6, tag: "div" },
            ReplaceWith { m: 5, root: 1 }
        ]
    );
}

/// Should result in the creation of an anchor (placeholder) and then a replacewith
/// Includes child nodes inside the fragment
#[test]
fn empty_fragments_create_anchors_with_many_children() {
    let dom = new_dom();

    let left = rsx!({ (0..0).map(|_| rsx! { div {} }) });
    let right = rsx!({
        (0..3).map(|f| {
            rsx! { div { "hello: {f}" }}
        })
    });

    let (create, change) = dom.diff_lazynodes(left, right);
    assert_eq!(
        create.edits,
        [CreatePlaceholder { root: 1 }, AppendChildren { many: 1 }]
    );

    assert_eq!(
        change.edits,
        [
            CreateElement { tag: "div", root: 2 },
            CreateTextNode { text: "hello: 0", root: 3 },
            AppendChildren { many: 1 },
            CreateElement { tag: "div", root: 4 },
            CreateTextNode { text: "hello: 1", root: 5 },
            AppendChildren { many: 1 },
            CreateElement { tag: "div", root: 6 },
            CreateTextNode { text: "hello: 2", root: 7 },
            AppendChildren { many: 1 },
            ReplaceWith { root: 1, m: 3 }
        ]
    );
}

/// Should result in every node being pushed and then replaced with an anchor
#[test]
fn many_items_become_fragment() {
    let dom = new_dom();

    let left = rsx!({
        (0..2).map(|_| {
            rsx! { div { "hello" }}
        })
    });
    let right = rsx!({ (0..0).map(|_| rsx! { div {} }) });

    let (create, change) = dom.diff_lazynodes(left, right);
    assert_eq!(
        create.edits,
        [
            CreateElement { root: 1, tag: "div" },
            CreateTextNode { text: "hello", root: 2 },
            AppendChildren { many: 1 },
            CreateElement { root: 3, tag: "div" },
            CreateTextNode { text: "hello", root: 4 },
            AppendChildren { many: 1 },
            AppendChildren { many: 2 },
        ]
    );

    assert_eq!(
        change.edits,
        [
            CreatePlaceholder { root: 5 },
            ReplaceWith { root: 1, m: 1 },
            Remove { root: 3 },
        ]
    );
}

/// Should result in no edits
#[test]
fn two_equal_fragments_are_equal() {
    let dom = new_dom();

    let left = rsx!({
        (0..2).map(|_| {
            rsx! { div { "hello" }}
        })
    });
    let right = rsx!({
        (0..2).map(|_| {
            rsx! { div { "hello" }}
        })
    });

    let (_create, change) = dom.diff_lazynodes(left, right);
    assert!(change.edits.is_empty());
}

/// Should result the creation of more nodes appended after the old last node
#[test]
fn two_fragments_with_differrent_elements_are_differet() {
    let dom = new_dom();

    let left = rsx!(
        { (0..2).map(|_| rsx! { div {  }} ) }
        p {}
    );
    let right = rsx!(
        { (0..5).map(|_| rsx! (h1 {  }) ) }
        p {}
    );

    let (_create, changes) = dom.diff_lazynodes(left, right);
    println!("{:#?}", &changes);
    assert_eq!(
        changes.edits,
        [
            // create the new h1s
            CreateElement { tag: "h1", root: 4 },
            CreateElement { tag: "h1", root: 5 },
            CreateElement { tag: "h1", root: 6 },
            InsertAfter { root: 2, n: 3 },
            // replace the divs with new h1s
            CreateElement { tag: "h1", root: 7 },
            ReplaceWith { root: 1, m: 1 },
            CreateElement { tag: "h1", root: 1 }, // notice how 1 gets re-used
            ReplaceWith { root: 2, m: 1 },
        ]
    );
}

/// Should result in multiple nodes destroyed - with changes to the first nodes
#[test]
fn two_fragments_with_differrent_elements_are_differet_shorter() {
    let dom = new_dom();

    let left = rsx!(
        {(0..5).map(|f| {rsx! { div {  }}})}
        p {}
    );
    let right = rsx!(
        {(0..2).map(|f| {rsx! { h1 {  }}})}
        p {}
    );

    let (create, change) = dom.diff_lazynodes(left, right);
    assert_eq!(
        create.edits,
        [
            CreateElement { root: 1, tag: "div" },
            CreateElement { root: 2, tag: "div" },
            CreateElement { root: 3, tag: "div" },
            CreateElement { root: 4, tag: "div" },
            CreateElement { root: 5, tag: "div" },
            CreateElement { root: 6, tag: "p" },
            AppendChildren { many: 6 },
        ]
    );

    // note: key reuse is always the last node that got used
    // slab maintains a linked list, essentially
    assert_eq!(
        change.edits,
        [
            Remove { root: 3 },
            Remove { root: 4 },
            Remove { root: 5 },
            CreateElement { root: 5, tag: "h1" }, // 3 gets reused
            ReplaceWith { root: 1, m: 1 },        // 1 gets deleted
            CreateElement { root: 1, tag: "h1" }, // 1 gets reused
            ReplaceWith { root: 2, m: 1 },
        ]
    );
}

/// Should result in multiple nodes destroyed - with no changes
#[test]
fn two_fragments_with_same_elements_are_differet() {
    let dom = new_dom();

    let left = rsx!(
        {(0..2).map(|f| {rsx! { div {  }}})}
        p {}
    );
    let right = rsx!(
        {(0..5).map(|f| {rsx! { div {  }}})}
        p {}
    );

    let (create, change) = dom.diff_lazynodes(left, right);
    assert_eq!(
        create.edits,
        [
            CreateElement { root: 1, tag: "div" },
            CreateElement { root: 2, tag: "div" },
            CreateElement { root: 3, tag: "p" },
            AppendChildren { many: 3 },
        ]
    );
    assert_eq!(
        change.edits,
        [
            CreateElement { root: 4, tag: "div" },
            CreateElement { root: 5, tag: "div" },
            CreateElement { root: 6, tag: "div" },
            InsertAfter { root: 2, n: 3 },
        ]
    );
}

/// should result in the removal of elements
#[test]
fn keyed_diffing_order() {
    let dom = new_dom();

    let left = rsx!(
        {(0..5).map(|f| {rsx! { div { key: "{f}"  }}})}
        p {"e"}
    );
    let right = rsx!(
        {(0..2).map(|f| rsx! { div { key: "{f}" }})}
        p {"e"}
    );

    let (create, change) = dom.diff_lazynodes(left, right);
    assert_eq!(
        change.edits,
        [Remove { root: 3 }, Remove { root: 4 }, Remove { root: 5 },]
    );
}

/// Should result in moves, but not removals or additions
#[test]
fn keyed_diffing_out_of_order() {
    let dom = new_dom();

    let left = rsx!({
        [0, 1, 2, 3, /**/ 4, 5, 6, /**/ 7, 8, 9].iter().map(|f| {
            rsx! { div { key: "{f}"  }}
        })
    });

    let right = rsx!({
        [0, 1, 2, 3, /**/ 6, 4, 5, /**/ 7, 8, 9].iter().map(|f| {
            rsx! { div { key: "{f}"  }}
        })
    });

    let (_, changes) = dom.diff_lazynodes(left, right);
    println!("{:?}", &changes);
    assert_eq!(
        changes.edits,
        [PushRoot { root: 7 }, InsertBefore { root: 5, n: 1 }]
    );
}

/// Should result in moves only
#[test]
fn keyed_diffing_out_of_order_adds() {
    let dom = new_dom();

    let left = rsx!({
        [/**/ 4, 5, 6, 7, 8 /**/].iter().map(|f| {
            rsx! { div { key: "{f}"  }}
        })
    });

    let right = rsx!({
        [/**/ 8, 7, 4, 5, 6 /**/].iter().map(|f| {
            rsx! { div { key: "{f}"  }}
        })
    });

    let (_, change) = dom.diff_lazynodes(left, right);
    assert_eq!(
        change.edits,
        [
            PushRoot { root: 5 },
            PushRoot { root: 4 },
            InsertBefore { n: 2, root: 1 }
        ]
    );
}
/// Should result in moves onl
#[test]
fn keyed_diffing_out_of_order_adds_2() {
    let dom = new_dom();

    let left = rsx!({
        [/**/ 4, 5, 6, 7, 8 /**/].iter().map(|f| {
            rsx! { div { key: "{f}"  }}
        })
    });

    let right = rsx!({
        [/**/ 7, 8, 4, 5, 6 /**/].iter().map(|f| {
            rsx! { div { key: "{f}"  }}
        })
    });

    let (_, change) = dom.diff_lazynodes(left, right);
    assert_eq!(
        change.edits,
        [
            PushRoot { root: 4 },
            PushRoot { root: 5 },
            InsertBefore { n: 2, root: 1 }
        ]
    );
}

/// Should result in moves onl
#[test]
fn keyed_diffing_out_of_order_adds_3() {
    let dom = new_dom();

    let left = rsx!({
        [/**/ 4, 5, 6, 7, 8 /**/].iter().map(|f| {
            rsx! { div { key: "{f}"  }}
        })
    });

    let right = rsx!({
        [/**/ 4, 8, 7, 5, 6 /**/].iter().map(|f| {
            rsx! { div { key: "{f}"  }}
        })
    });

    let (_, change) = dom.diff_lazynodes(left, right);
    assert_eq!(
        change.edits,
        [
            PushRoot { root: 5 },
            PushRoot { root: 4 },
            InsertBefore { n: 2, root: 2 }
        ]
    );
}

/// Should result in moves onl
#[test]
fn keyed_diffing_out_of_order_adds_4() {
    let dom = new_dom();

    let left = rsx!({
        [/**/ 4, 5, 6, 7, 8 /**/].iter().map(|f| {
            rsx! { div { key: "{f}"  }}
        })
    });

    let right = rsx!({
        [/**/ 4, 5, 8, 7, 6 /**/].iter().map(|f| {
            rsx! { div { key: "{f}"  }}
        })
    });

    let (_, change) = dom.diff_lazynodes(left, right);
    assert_eq!(
        change.edits,
        [
            PushRoot { root: 5 },
            PushRoot { root: 4 },
            InsertBefore { n: 2, root: 3 }
        ]
    );
}

/// Should result in moves onl
#[test]
fn keyed_diffing_out_of_order_adds_5() {
    let dom = new_dom();

    let left = rsx!({
        [/**/ 4, 5, 6, 7, 8 /**/].iter().map(|f| {
            rsx! { div { key: "{f}"  }}
        })
    });

    let right = rsx!({
        [/**/ 4, 5, 6, 8, 7 /**/].iter().map(|f| {
            rsx! { div { key: "{f}"  }}
        })
    });

    let (_, change) = dom.diff_lazynodes(left, right);
    assert_eq!(
        change.edits,
        [PushRoot { root: 5 }, InsertBefore { n: 1, root: 4 }]
    );
}

#[test]
fn keyed_diffing_additions() {
    let dom = new_dom();

    let left = rsx!({
        [/**/ 4, 5, 6, 7, 8 /**/].iter().map(|f| {
            rsx! { div { key: "{f}"  }}
        })
    });

    let right = rsx!({
        [/**/ 4, 5, 6, 7, 8, 9, 10 /**/].iter().map(|f| {
            rsx! { div { key: "{f}"  }}
        })
    });

    let (_, change) = dom.diff_lazynodes(left, right);
    assert_eq!(
        change.edits,
        [
            CreateElement { root: 6, tag: "div" },
            CreateElement { root: 7, tag: "div" },
            InsertAfter { n: 2, root: 5 }
        ]
    );
}

#[test]
fn keyed_diffing_additions_and_moves_on_ends() {
    let dom = new_dom();

    let left = rsx!({
        [/**/ 4, 5, 6, 7 /**/].iter().map(|f| {
            rsx! { div { key: "{f}"  }}
        })
    });

    let right = rsx!({
        [/**/ 7, 4, 5, 6, 11, 12 /**/].iter().map(|f| {
            rsx! { div { key: "{f}"  }}
        })
    });

    let (_, change) = dom.diff_lazynodes(left, right);
    println!("{:?}", change);
    assert_eq!(
        change.edits,
        [
            // create 11, 12
            CreateElement { tag: "div", root: 5 },
            CreateElement { tag: "div", root: 6 },
            InsertAfter { root: 3, n: 2 },
            // move 7 to the front
            PushRoot { root: 4 },
            InsertBefore { root: 1, n: 1 }
        ]
    );
}

#[test]
fn keyed_diffing_additions_and_moves_in_middle() {
    let dom = new_dom();

    let left = rsx!({
        [/**/ 1, 2, 3, 4 /**/].iter().map(|f| {
            rsx! { div { key: "{f}"  }}
        })
    });

    let right = rsx!({
        [/**/ 4, 1, 7, 8, 2, 5, 6, 3 /**/].iter().map(|f| {
            rsx! { div { key: "{f}"  }}
        })
    });

    // LIS: 4, 5, 6
    let (_, change) = dom.diff_lazynodes(left, right);
    println!("{:#?}", change);
    assert_eq!(
        change.edits,
        [
            // create 5, 6
            CreateElement { tag: "div", root: 5 },
            CreateElement { tag: "div", root: 6 },
            InsertBefore { root: 3, n: 2 },
            // create 7, 8
            CreateElement { tag: "div", root: 7 },
            CreateElement { tag: "div", root: 8 },
            InsertBefore { root: 2, n: 2 },
            // move 7
            PushRoot { root: 4 },
            InsertBefore { root: 1, n: 1 }
        ]
    );
}

#[test]
fn controlled_keyed_diffing_out_of_order() {
    let dom = new_dom();

    let left = rsx!({
        [4, 5, 6, 7].iter().map(|f| {
            rsx! { div { key: "{f}" }}
        })
    });

    let right = rsx!({
        [0, 5, 9, 6, 4].iter().map(|f| {
            rsx! { div { key: "{f}" }}
        })
    });

    // LIS: 5, 6
    let (_, changes) = dom.diff_lazynodes(left, right);
    println!("{:#?}", &changes);
    assert_eq!(
        changes.edits,
        [
            Remove { root: 4 },
            // move 4 to after 6
            PushRoot { root: 1 },
            InsertAfter { n: 1, root: 3 },
            // remove 7

            // create 9 and insert before 6
            CreateElement { root: 4, tag: "div" },
            InsertBefore { n: 1, root: 3 },
            // create 0 and insert before 5
            CreateElement { root: 5, tag: "div" },
            InsertBefore { n: 1, root: 2 },
        ]
    );
}

#[test]
fn controlled_keyed_diffing_out_of_order_max_test() {
    let dom = new_dom();

    let left = rsx!({
        [0, 1, 2, 3, 4].iter().map(|f| {
            rsx! { div { key: "{f}"  }}
        })
    });

    let right = rsx!({
        [3, 0, 1, 10, 2].iter().map(|f| {
            rsx! { div { key: "{f}"  }}
        })
    });

    let (_, changes) = dom.diff_lazynodes(left, right);
    println!("{:#?}", &changes);
    assert_eq!(
        changes.edits,
        [
            Remove { root: 5 },
            CreateElement { root: 5, tag: "div" },
            InsertBefore { n: 1, root: 3 },
            PushRoot { root: 4 },
            InsertBefore { n: 1, root: 1 },
        ]
    );
}

// noticed some weird behavior in the desktop interpreter
// just making sure it doesnt happen in the core implementation
#[test]
fn remove_list() {
    let dom = new_dom();

    let left = rsx!({
        (0..10).rev().take(5).map(|i| {
            rsx! { Fragment { key: "{i}", "{i}" }}
        })
    });

    let right = rsx!({
        (0..10).rev().take(2).map(|i| {
            rsx! { Fragment { key: "{i}", "{i}" }}
        })
    });

    let (create, changes) = dom.diff_lazynodes(left, right);

    // dbg!(create);
    // dbg!(changes);

    assert_eq!(
        changes.edits,
        [
            // remove 5, 4, 3
            Remove { root: 3 },
            Remove { root: 4 },
            Remove { root: 5 },
        ]
    );
}

// noticed some weird behavior in the desktop interpreter
// just making sure it doesnt happen in the core implementation
#[test]
fn remove_list_nokeyed() {
    let dom = new_dom();

    let left = rsx!({
        (0..10).rev().take(5).map(|i| {
            rsx! { Fragment { "{i}" }}
        })
    });

    let right = rsx!({
        (0..10).rev().take(2).map(|i| {
            rsx! { Fragment { "{i}" }}
        })
    });

    let (create, changes) = dom.diff_lazynodes(left, right);

    assert_eq!(
        changes.edits,
        [
            // remove 5, 4, 3
            Remove { root: 3 },
            Remove { root: 4 },
            Remove { root: 5 },
        ]
    );
}

#[test]
fn add_nested_elements() {
    let vdom = new_dom();

    let (_create, change) = vdom.diff_lazynodes(
        rsx! {
            div{}
        },
        rsx! {
            div{
                div{}
            }
        },
    );

    assert_eq!(
        change.edits,
        [
            PushRoot { root: 1 },
            CreateElement { root: 2, tag: "div" },
            AppendChildren { many: 1 },
            PopRoot {},
        ]
    );
}

#[test]
fn add_listeners() {
    let vdom = new_dom();

    let (_create, change) = vdom.diff_lazynodes(
        rsx! {
            div{}
        },
        rsx! {
            div{
                onkeyup: |_| {},
                onkeydown: |_| {},
            }
        },
    );

    assert_eq!(
        change.edits,
        [
            NewEventListener { event_name: "keyup", scope: ScopeId(0), root: 1 },
            NewEventListener { event_name: "keydown", scope: ScopeId(0), root: 1 },
        ]
    );
}

#[test]
fn remove_listeners() {
    let vdom = new_dom();

    let (_create, change) = vdom.diff_lazynodes(
        rsx! {
            div{
                onkeyup: |_| {},
                onkeydown: |_| {},
            }
        },
        rsx! {
            div{}
        },
    );

    assert_eq!(
        change.edits,
        [
            RemoveEventListener { event: "keyup", root: 1 },
            RemoveEventListener { event: "keydown", root: 1 },
        ]
    );
}

#[test]
fn diff_listeners() {
    let vdom = new_dom();

    let (_create, change) = vdom.diff_lazynodes(
        rsx! {
            div{
                onkeydown: |_| {},
            }
        },
        rsx! {
            div{
                onkeyup: |_| {},
            }
        },
    );

    assert_eq!(
        change.edits,
        [
            RemoveEventListener { root: 1, event: "keydown" },
            NewEventListener { event_name: "keyup", scope: ScopeId(0), root: 1 }
        ]
    );
}
