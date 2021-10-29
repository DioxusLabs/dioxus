#![allow(unused, non_upper_case_globals)]
//! Diffing Tests
//!
//! These tests only verify that the diffing algorithm works properly for single components.
//!
//! It does not validated that component lifecycles work properly. This is done in another test file.

use dioxus::{nodes::VSuspended, prelude::*, DomEdit, TestDom};
use dioxus_core as dioxus;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

mod test_logging;

fn new_dom() -> TestDom {
    const IS_LOGGING_ENABLED: bool = false;
    test_logging::set_up_logging(IS_LOGGING_ENABLED);
    TestDom::new()
}

use DomEdit::*;

/// Should push the text node onto the stack and modify it
#[test]
fn html_and_rsx_generate_the_same_output() {
    let dom = new_dom();
    let (create, change) = dom.lazy_diff(
        rsx! ( div { "Hello world" } ),
        rsx! ( div { "Goodbye world" } ),
    );
    assert_eq!(
        create.edits,
        [
            CreateElement {
                root: 0,
                tag: "div"
            },
            CreateTextNode {
                root: 1,
                text: "Hello world"
            },
            AppendChildren { many: 1 },
            AppendChildren { many: 1 },
        ]
    );

    assert_eq!(
        change.edits,
        [SetText {
            text: "Goodbye world",
            root: 1
        },]
    );
}

/// Should result in 3 elements on the stack
#[test]
fn fragments_create_properly() {
    let dom = new_dom();

    let create = dom.create(rsx! {
        div { "Hello a" }
        div { "Hello b" }
        div { "Hello c" }
    });

    assert_eq!(
        create.edits,
        [
            CreateElement {
                root: 0,
                tag: "div"
            },
            CreateTextNode {
                root: 1,
                text: "Hello a"
            },
            AppendChildren { many: 1 },
            CreateElement {
                root: 2,
                tag: "div"
            },
            CreateTextNode {
                root: 3,
                text: "Hello b"
            },
            AppendChildren { many: 1 },
            CreateElement {
                root: 4,
                tag: "div"
            },
            CreateTextNode {
                root: 5,
                text: "Hello c"
            },
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

    let (create, change) = dom.lazy_diff(left, right);

    assert_eq!(
        create.edits,
        [CreatePlaceholder { root: 0 }, AppendChildren { many: 1 }]
    );
    assert_eq!(
        change.edits,
        [
            CreateElement {
                root: 1,
                tag: "div"
            },
            ReplaceWith { m: 1, root: 0 }
        ]
    );
}

/// Should result in the creation of an anchor (placeholder) and then a replacewith m=5
#[test]
fn empty_fragments_create_many_anchors() {
    let dom = new_dom();

    let left = rsx!({ (0..0).map(|_f| rsx! { div {}}) });
    let right = rsx!({ (0..5).map(|_f| rsx! { div {}}) });

    let (create, change) = dom.lazy_diff(left, right);
    assert_eq!(
        create.edits,
        [CreatePlaceholder { root: 0 }, AppendChildren { many: 1 }]
    );
    assert_eq!(
        change.edits,
        [
            CreateElement {
                root: 1,
                tag: "div"
            },
            CreateElement {
                root: 2,
                tag: "div"
            },
            CreateElement {
                root: 3,
                tag: "div"
            },
            CreateElement {
                root: 4,
                tag: "div"
            },
            CreateElement {
                root: 5,
                tag: "div"
            },
            ReplaceWith { m: 5, root: 0 }
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

    let (create, change) = dom.lazy_diff(left, right);
    assert_eq!(
        create.edits,
        [CreatePlaceholder { root: 0 }, AppendChildren { many: 1 }]
    );
    assert_eq!(
        change.edits,
        [
            CreateElement {
                root: 1,
                tag: "div"
            },
            CreateTextNode {
                text: "hello: 0",
                root: 2
            },
            AppendChildren { many: 1 },
            CreateElement {
                root: 3,
                tag: "div"
            },
            CreateTextNode {
                text: "hello: 1",
                root: 4
            },
            AppendChildren { many: 1 },
            CreateElement {
                root: 5,
                tag: "div"
            },
            CreateTextNode {
                text: "hello: 2",
                root: 6
            },
            AppendChildren { many: 1 },
            ReplaceWith { m: 3, root: 0 }
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

    let (create, change) = dom.lazy_diff(left, right);
    assert_eq!(
        create.edits,
        [
            CreateElement {
                root: 0,
                tag: "div"
            },
            CreateTextNode {
                text: "hello",
                root: 1
            },
            AppendChildren { many: 1 },
            CreateElement {
                root: 2,
                tag: "div"
            },
            CreateTextNode {
                text: "hello",
                root: 3
            },
            AppendChildren { many: 1 },
            AppendChildren { many: 2 },
        ]
    );

    // hmmmmmmmmm worried about reusing IDs that we shouldnt be
    assert_eq!(
        change.edits,
        [
            Remove { root: 2 },
            CreatePlaceholder { root: 4 },
            ReplaceWith { root: 0, m: 1 },
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

    let (_create, change) = dom.lazy_diff(left, right);
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

    let (_create, changes) = dom.lazy_diff(left, right);
    log::debug!("{:#?}", &changes);
    assert_eq!(
        changes.edits,
        [
            // create the new h1s
            CreateElement { tag: "h1", root: 3 },
            CreateElement { tag: "h1", root: 4 },
            CreateElement { tag: "h1", root: 5 },
            InsertAfter { root: 1, n: 3 },
            // replace the divs with new h1s
            CreateElement { tag: "h1", root: 6 },
            ReplaceWith { root: 0, m: 1 },
            CreateElement { tag: "h1", root: 7 },
            ReplaceWith { root: 1, m: 1 },
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

    let (create, change) = dom.lazy_diff(left, right);
    assert_eq!(
        create.edits,
        [
            CreateElement {
                root: 0,
                tag: "div"
            },
            CreateElement {
                root: 1,
                tag: "div"
            },
            CreateElement {
                root: 2,
                tag: "div"
            },
            CreateElement {
                root: 3,
                tag: "div"
            },
            CreateElement {
                root: 4,
                tag: "div"
            },
            CreateElement { root: 5, tag: "p" },
            AppendChildren { many: 6 },
        ]
    );
    assert_eq!(
        change.edits,
        [
            Remove { root: 2 },
            Remove { root: 3 },
            Remove { root: 4 },
            CreateElement { root: 6, tag: "h1" },
            ReplaceWith { root: 0, m: 1 },
            CreateElement { root: 7, tag: "h1" },
            ReplaceWith { root: 1, m: 1 },
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

    let (create, change) = dom.lazy_diff(left, right);
    assert_eq!(
        create.edits,
        [
            CreateElement {
                root: 0,
                tag: "div"
            },
            CreateElement {
                root: 1,
                tag: "div"
            },
            CreateElement { root: 2, tag: "p" },
            AppendChildren { many: 3 },
        ]
    );
    assert_eq!(
        change.edits,
        [
            CreateElement {
                root: 3,
                tag: "div"
            },
            CreateElement {
                root: 4,
                tag: "div"
            },
            CreateElement {
                root: 5,
                tag: "div"
            },
            InsertAfter { root: 1, n: 3 },
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
        {(0..2).map(|f| {rsx! { div { key: "{f}" }}})}
        p {"e"}
    );

    let (create, change) = dom.lazy_diff(left, right);
    assert_eq!(
        change.edits,
        [Remove { root: 2 }, Remove { root: 3 }, Remove { root: 4 },]
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

    let (_, changes) = dom.lazy_diff(left, right);
    log::debug!("{:?}", &changes);
    assert_eq!(
        changes.edits,
        [PushRoot { root: 6 }, InsertBefore { root: 4, n: 1 }]
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

    let (_, change) = dom.lazy_diff(left, right);
    assert_eq!(
        change.edits,
        [
            PushRoot { root: 4 },
            PushRoot { root: 3 },
            InsertBefore { n: 2, root: 0 }
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

    let (_, change) = dom.lazy_diff(left, right);
    assert_eq!(
        change.edits,
        [
            PushRoot { root: 3 },
            PushRoot { root: 4 },
            InsertBefore { n: 2, root: 0 }
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

    let (_, change) = dom.lazy_diff(left, right);
    assert_eq!(
        change.edits,
        [
            PushRoot { root: 4 },
            PushRoot { root: 3 },
            InsertBefore { n: 2, root: 1 }
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

    let (_, change) = dom.lazy_diff(left, right);
    assert_eq!(
        change.edits,
        [
            PushRoot { root: 4 },
            PushRoot { root: 3 },
            InsertBefore { n: 2, root: 2 }
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

    let (_, change) = dom.lazy_diff(left, right);
    assert_eq!(
        change.edits,
        [PushRoot { root: 4 }, InsertBefore { n: 1, root: 3 }]
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

    let (_, change) = dom.lazy_diff(left, right);
    assert_eq!(
        change.edits,
        [
            CreateElement {
                root: 5,
                tag: "div"
            },
            CreateElement {
                root: 6,
                tag: "div"
            },
            InsertAfter { n: 2, root: 4 }
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

    let (_, change) = dom.lazy_diff(left, right);
    log::debug!("{:?}", change);
    assert_eq!(
        change.edits,
        [
            // create 11, 12
            CreateElement {
                tag: "div",
                root: 4
            },
            CreateElement {
                tag: "div",
                root: 5
            },
            InsertAfter { root: 2, n: 2 },
            // move 7 to the front
            PushRoot { root: 3 },
            InsertBefore { root: 0, n: 1 }
        ]
    );
}

#[test]
fn keyed_diffing_additions_and_moves_in_middle() {
    let dom = new_dom();

    let left = rsx!({
        [/**/ 4, 5, 6, 7 /**/].iter().map(|f| {
            rsx! { div { key: "{f}"  }}
        })
    });

    let right = rsx!({
        [/**/ 7, 4, 13, 17, 5, 11, 12, 6 /**/].iter().map(|f| {
            rsx! { div { key: "{f}"  }}
        })
    });

    // LIS: 4, 5, 6
    let (_, change) = dom.lazy_diff(left, right);
    log::debug!("{:#?}", change);
    assert_eq!(
        change.edits,
        [
            // create 13, 17
            CreateElement {
                tag: "div",
                root: 4
            },
            CreateElement {
                tag: "div",
                root: 5
            },
            InsertBefore { root: 1, n: 2 },
            // create 11, 12
            CreateElement {
                tag: "div",
                root: 6
            },
            CreateElement {
                tag: "div",
                root: 7
            },
            InsertBefore { root: 2, n: 2 },
            // move 7
            PushRoot { root: 3 },
            InsertBefore { root: 0, n: 1 }
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
    let (_, changes) = dom.lazy_diff(left, right);
    log::debug!("{:#?}", &changes);
    assert_eq!(
        changes.edits,
        [
            // move 4 to after 6
            PushRoot { root: 0 },
            InsertAfter { n: 1, root: 2 },
            // remove 7

            // create 9 and insert before 6
            CreateElement {
                root: 4,
                tag: "div"
            },
            InsertBefore { n: 1, root: 2 },
            // create 0 and insert before 5
            CreateElement {
                root: 5,
                tag: "div"
            },
            InsertBefore { n: 1, root: 1 },
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

    let (_, changes) = dom.lazy_diff(left, right);
    log::debug!("{:#?}", &changes);
    assert_eq!(
        changes.edits,
        [
            CreateElement {
                root: 5,
                tag: "div"
            },
            InsertBefore { n: 1, root: 2 },
            PushRoot { root: 3 },
            InsertBefore { n: 1, root: 0 },
        ]
    );
}

#[test]
fn suspense() {
    let dom = new_dom();

    let edits = dom.create(Some(LazyNodes::new(|f| {
        use std::cell::{Cell, RefCell};
        VNode::Suspended(f.bump().alloc(VSuspended {
            task_id: 0,
            callback: RefCell::new(None),
            dom_id: Cell::new(None),
        }))
    })));
    assert_eq!(
        edits.edits,
        [CreatePlaceholder { root: 0 }, AppendChildren { many: 1 }]
    );
}
