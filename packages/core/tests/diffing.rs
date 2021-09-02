//! Diffing Tests

use bumpalo::Bump;

use dioxus::{
    diff::DiffMachine, prelude::*, scheduler::Scheduler, DiffInstruction, DomEdit, MountType,
};
use dioxus_core as dioxus;
use dioxus_html as dioxus_elements;
use futures_util::FutureExt;
mod test_logging;
use DomEdit::*;

// logging is wired up to the test harness
// feel free to enable while debugging
const IS_LOGGING_ENABLED: bool = false;

#[test]
fn diffing_works() {}

/// Should push the text node onto the stack and modify it
#[test]
fn html_and_rsx_generate_the_same_output() {
    let dom = TestDom::new();
    let (create, change) = dom.lazy_diff(
        rsx! ( div { "Hello world" } ),
        rsx! ( div { "Goodbye world" } ),
    );
    assert_eq!(
        create.edits,
        [
            CreateElement { id: 0, tag: "div" },
            CreateTextNode {
                id: 1,
                text: "Hello world"
            },
            AppendChildren { many: 1 },
            AppendChildren { many: 1 },
        ]
    );

    assert_eq!(
        change.edits,
        [
            PushRoot { id: 1 },
            SetText {
                text: "Goodbye world"
            },
            PopRoot
        ]
    );
}

/// Should result in 3 elements on the stack
#[test]
fn fragments_create_properly() {
    let dom = TestDom::new();

    let create = dom.create(rsx! {
        div { "Hello a" }
        div { "Hello b" }
        div { "Hello c" }
    });

    assert_eq!(
        create.edits,
        [
            CreateElement { id: 0, tag: "div" },
            CreateTextNode {
                id: 1,
                text: "Hello a"
            },
            AppendChildren { many: 1 },
            CreateElement { id: 2, tag: "div" },
            CreateTextNode {
                id: 3,
                text: "Hello b"
            },
            AppendChildren { many: 1 },
            CreateElement { id: 4, tag: "div" },
            CreateTextNode {
                id: 5,
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
    let dom = TestDom::new();

    let left = rsx!({ (0..0).map(|f| rsx! { div {}}) });
    let right = rsx!({ (0..1).map(|f| rsx! { div {}}) });

    let (create, change) = dom.lazy_diff(left, right);

    assert_eq!(
        create.edits,
        [CreatePlaceholder { id: 0 }, AppendChildren { many: 1 }]
    );
    assert_eq!(
        change.edits,
        [
            CreateElement { id: 1, tag: "div" },
            ReplaceWith { m: 1, root: 0 }
        ]
    );
}

/// Should result in the creation of an anchor (placeholder) and then a replacewith m=5
#[test]
fn empty_fragments_create_many_anchors() {
    let dom = TestDom::new();

    let left = rsx!({ (0..0).map(|f| rsx! { div {}}) });
    let right = rsx!({ (0..5).map(|f| rsx! { div {}}) });

    let (create, change) = dom.lazy_diff(left, right);
    assert_eq!(
        create.edits,
        [CreatePlaceholder { id: 0 }, AppendChildren { many: 1 }]
    );
    assert_eq!(
        change.edits,
        [
            CreateElement { id: 1, tag: "div" },
            CreateElement { id: 2, tag: "div" },
            CreateElement { id: 3, tag: "div" },
            CreateElement { id: 4, tag: "div" },
            CreateElement { id: 5, tag: "div" },
            ReplaceWith { m: 5, root: 0 }
        ]
    );
}

/// Should result in the creation of an anchor (placeholder) and then a replacewith
/// Includes child nodes inside the fragment
#[test]
fn empty_fragments_create_anchors_with_many_children() {
    let dom = TestDom::new();

    let left = rsx!({ (0..0).map(|f| rsx! { div {} }) });
    let right = rsx!({
        (0..3).map(|f| {
            rsx! { div { "hello: {f}" }}
        })
    });

    let (create, change) = dom.lazy_diff(left, right);
    assert_eq!(
        create.edits,
        [CreatePlaceholder { id: 0 }, AppendChildren { many: 1 }]
    );
    assert_eq!(
        change.edits,
        [
            CreateElement { id: 1, tag: "div" },
            CreateTextNode {
                text: "hello: 0",
                id: 2
            },
            AppendChildren { many: 1 },
            CreateElement { id: 3, tag: "div" },
            CreateTextNode {
                text: "hello: 1",
                id: 4
            },
            AppendChildren { many: 1 },
            CreateElement { id: 5, tag: "div" },
            CreateTextNode {
                text: "hello: 2",
                id: 6
            },
            AppendChildren { many: 1 },
            ReplaceWith { m: 3, root: 0 }
        ]
    );
}

/// Should result in every node being pushed and then replaced with an anchor
#[test]
fn many_items_become_fragment() {
    let dom = TestDom::new();

    let left = rsx!({
        (0..2).map(|f| {
            rsx! { div { "hello" }}
        })
    });
    let right = rsx!({ (0..0).map(|f| rsx! { div {} }) });

    let (create, change) = dom.lazy_diff(left, right);
    assert_eq!(
        create.edits,
        [
            CreateElement { id: 0, tag: "div" },
            CreateTextNode {
                text: "hello",
                id: 1
            },
            AppendChildren { many: 1 },
            CreateElement { id: 2, tag: "div" },
            CreateTextNode {
                text: "hello",
                id: 3
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
            CreatePlaceholder { id: 4 },
            ReplaceWith { root: 0, m: 1 },
        ]
    );
}

/// Should result in no edits
#[test]
fn two_equal_fragments_are_equal() {
    let dom = TestDom::new();

    let left = rsx!({
        (0..2).map(|f| {
            rsx! { div { "hello" }}
        })
    });
    let right = rsx!({
        (0..2).map(|f| {
            rsx! { div { "hello" }}
        })
    });

    let (create, change) = dom.lazy_diff(left, right);
    assert!(change.edits.is_empty());
}

/// Should result the creation of more nodes appended after the old last node
#[test]
fn two_fragments_with_differrent_elements_are_differet() {
    let dom = TestDom::new();

    let left = rsx!(
        { (0..2).map(|_| rsx! { div {  }} ) }
        p {}
    );
    let right = rsx!(
        { (0..5).map(|_| rsx! (h1 {  }) ) }
        p {}
    );

    let (create, changes) = dom.lazy_diff(left, right);
    log::debug!("{:#?}", &changes);
    assert_eq!(
        changes.edits,
        [
            // create the new h1s
            CreateElement { tag: "h1", id: 3 },
            CreateElement { tag: "h1", id: 4 },
            CreateElement { tag: "h1", id: 5 },
            InsertAfter { root: 1, n: 3 },
            // replace the divs with new h1s
            CreateElement { tag: "h1", id: 6 },
            ReplaceWith { root: 0, m: 1 },
            CreateElement { tag: "h1", id: 7 },
            ReplaceWith { root: 1, m: 1 },
        ]
    );
}

/// Should result in multiple nodes destroyed - with changes to the first nodes
#[test]
fn two_fragments_with_differrent_elements_are_differet_shorter() {
    let dom = TestDom::new();

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
            CreateElement { id: 0, tag: "div" },
            CreateElement { id: 1, tag: "div" },
            CreateElement { id: 2, tag: "div" },
            CreateElement { id: 3, tag: "div" },
            CreateElement { id: 4, tag: "div" },
            CreateElement { id: 5, tag: "p" },
            AppendChildren { many: 6 },
        ]
    );
    assert_eq!(
        change.edits,
        [
            Remove { root: 2 },
            Remove { root: 3 },
            Remove { root: 4 },
            CreateElement { id: 6, tag: "h1" },
            ReplaceWith { root: 0, m: 1 },
            CreateElement { id: 7, tag: "h1" },
            ReplaceWith { root: 1, m: 1 },
        ]
    );
}

/// Should result in multiple nodes destroyed - with no changes
#[test]
fn two_fragments_with_same_elements_are_differet() {
    let dom = TestDom::new();

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
            CreateElement { id: 0, tag: "div" },
            CreateElement { id: 1, tag: "div" },
            CreateElement { id: 2, tag: "p" },
            AppendChildren { many: 3 },
        ]
    );
    assert_eq!(
        change.edits,
        [
            CreateElement { id: 3, tag: "div" },
            CreateElement { id: 4, tag: "div" },
            CreateElement { id: 5, tag: "div" },
            InsertAfter { root: 1, n: 3 },
        ]
    );
}

/// should result in the removal of elements
#[test]
fn keyed_diffing_order() {
    let dom = TestDom::new();

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
    let dom = TestDom::new();

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
        [PushRoot { id: 6 }, InsertBefore { root: 4, n: 1 }]
    );
}

/// Should result in moves only
#[test]
fn keyed_diffing_out_of_order_adds() {
    let dom = TestDom::new();

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
            PushRoot { id: 4 },
            PushRoot { id: 3 },
            InsertBefore { n: 2, root: 0 }
        ]
    );
}
/// Should result in moves onl
#[test]
fn keyed_diffing_out_of_order_adds_2() {
    let dom = TestDom::new();

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
            PushRoot { id: 3 },
            PushRoot { id: 4 },
            InsertBefore { n: 2, root: 0 }
        ]
    );
}

/// Should result in moves onl
#[test]
fn keyed_diffing_out_of_order_adds_3() {
    let dom = TestDom::new();

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
            PushRoot { id: 4 },
            PushRoot { id: 3 },
            InsertBefore { n: 2, root: 1 }
        ]
    );
}

/// Should result in moves onl
#[test]
fn keyed_diffing_out_of_order_adds_4() {
    let dom = TestDom::new();

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
            PushRoot { id: 4 },
            PushRoot { id: 3 },
            InsertBefore { n: 2, root: 2 }
        ]
    );
}

/// Should result in moves onl
#[test]
fn keyed_diffing_out_of_order_adds_5() {
    let dom = TestDom::new();

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
        [PushRoot { id: 4 }, InsertBefore { n: 1, root: 3 }]
    );
}

#[test]
fn keyed_diffing_additions() {
    let dom = TestDom::new();

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
            CreateElement { id: 5, tag: "div" },
            CreateElement { id: 6, tag: "div" },
            InsertAfter { n: 2, root: 4 }
        ]
    );
}

#[test]
fn keyed_diffing_additions_and_moves_on_ends() {
    let dom = TestDom::new();

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
            CreateElement { tag: "div", id: 4 },
            CreateElement { tag: "div", id: 5 },
            InsertAfter { root: 2, n: 2 },
            // move 7 to the front
            PushRoot { id: 3 },
            InsertBefore { root: 0, n: 1 }
        ]
    );
}

#[test]
fn keyed_diffing_additions_and_moves_in_middle() {
    let dom = TestDom::new();

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
            CreateElement { tag: "div", id: 4 },
            CreateElement { tag: "div", id: 5 },
            InsertBefore { root: 1, n: 2 },
            // create 11, 12
            CreateElement { tag: "div", id: 6 },
            CreateElement { tag: "div", id: 7 },
            InsertBefore { root: 2, n: 2 },
            // move 7
            PushRoot { id: 3 },
            InsertBefore { root: 0, n: 1 }
        ]
    );
}

#[test]
fn controlled_keyed_diffing_out_of_order() {
    let dom = TestDom::new();

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
            PushRoot { id: 0 },
            InsertAfter { n: 1, root: 2 },
            // remove 7

            // create 9 and insert before 6
            CreateElement { id: 4, tag: "div" },
            InsertBefore { n: 1, root: 2 },
            // create 0 and insert before 5
            CreateElement { id: 5, tag: "div" },
            InsertBefore { n: 1, root: 1 },
        ]
    );
}

#[test]
fn controlled_keyed_diffing_out_of_order_max_test() {
    let dom = TestDom::new();

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
            CreateElement { id: 5, tag: "div" },
            InsertBefore { n: 1, root: 2 },
            PushRoot { id: 3 },
            InsertBefore { n: 1, root: 0 },
        ]
    );
}
