//! Diffing Tests
//! -------------
//!
//! These should always compile and run, but the result is not validated for each test.
//! TODO: Validate the results beyond visual inspection.

use bumpalo::Bump;

use anyhow::{Context, Result};
use dioxus::{
    arena::SharedResources,
    diff::{DiffInstruction, DiffMachine},
    prelude::*,
    DomEdit,
};
use dioxus_core as dioxus;
use dioxus_html as dioxus_elements;
use futures_util::FutureExt;

struct TestDom {
    bump: Bump,
    resources: SharedResources,
}
impl TestDom {
    fn new() -> TestDom {
        let bump = Bump::new();
        let resources = SharedResources::new();
        TestDom { bump, resources }
    }
    fn new_factory<'a>(&'a self) -> NodeFactory<'a> {
        NodeFactory::new(&self.bump)
    }

    fn render<'a, F>(&'a self, lazy_nodes: LazyNodes<'a, F>) -> VNode<'a>
    where
        F: FnOnce(NodeFactory<'a>) -> VNode<'a>,
    {
        use dioxus_core::nodes::{IntoVNode, IntoVNodeList};
        lazy_nodes.into_vnode(NodeFactory::new(&self.bump))
    }

    fn diff<'a>(&'a self, old: &'a VNode<'a>, new: &'a VNode<'a>) -> Mutations<'a> {
        // let mut edits = Vec::new();
        let mut machine = DiffMachine::new_headless(&self.resources);

        machine
            .stack
            .push(dioxus::diff::DiffInstruction::DiffNode { new, old });

        machine.mutations
    }

    fn create<'a, F1>(&'a self, left: LazyNodes<'a, F1>) -> Mutations<'a>
    where
        F1: FnOnce(NodeFactory<'a>) -> VNode<'a>,
    {
        let old = self.bump.alloc(self.render(left));

        let mut machine = DiffMachine::new_headless(&self.resources);

        machine.stack.push(dioxus::diff::DiffInstruction::Create {
            node: old,
            and: dioxus::diff::MountType::Append,
        });
        work_sync(&mut machine);

        machine.mutations
    }

    fn lazy_diff<'a, F1, F2>(
        &'a self,
        left: LazyNodes<'a, F1>,
        right: LazyNodes<'a, F2>,
    ) -> (Mutations<'a>, Mutations<'a>)
    where
        F1: FnOnce(NodeFactory<'a>) -> VNode<'a>,
        F2: FnOnce(NodeFactory<'a>) -> VNode<'a>,
    {
        let old = self.bump.alloc(self.render(left));

        let new = self.bump.alloc(self.render(right));

        // let mut create_edits = Vec::new();

        let mut machine = DiffMachine::new_headless(&self.resources);
        machine.stack.push(dioxus::diff::DiffInstruction::Create {
            and: dioxus::diff::MountType::Append,
            node: old,
        });
        work_sync(&mut machine);
        let create_edits = machine.mutations;

        let mut machine = DiffMachine::new_headless(&self.resources);
        machine.stack.push(DiffInstruction::DiffNode { old, new });
        work_sync(&mut machine);
        let edits = machine.mutations;

        (create_edits, edits)
    }
}

fn work_sync(machine: &mut DiffMachine) {
    let mut fut = machine.work().boxed_local();

    while let None = (&mut fut).now_or_never() {
        //
    }
}

#[test]
fn diffing_works() {}

/// Should push the text node onto the stack and modify it
#[test]
fn html_and_rsx_generate_the_same_output() {
    let dom = TestDom::new();
    let edits = dom.lazy_diff(
        rsx! ( div { "Hello world" } ),
        rsx! ( div { "Goodbye world" } ),
    );
    dbg!(edits);
}

/// Should result in 3 elements on the stack
#[test]
fn fragments_create_properly() {
    let dom = TestDom::new();
    let Mutations { edits, noderefs } = dom.create(rsx! {
        div { "Hello a" }
        div { "Hello b" }
        div { "Hello c" }
    });
    assert!(&edits[0].is("CreateElement"));
    assert!(&edits[3].is("CreateElement"));
    assert!(&edits[6].is("CreateElement"));

    assert_eq!(*edits.last().unwrap(), DomEdit::AppendChildren { many: 3 });
    dbg!(edits);
}

/// Should result in the creation of an anchor (placeholder) and then a replacewith
#[test]
fn empty_fragments_create_anchors() {
    let dom = TestDom::new();

    let left = rsx!({ (0..0).map(|f| rsx! { div {}}) });
    let right = rsx!({ (0..1).map(|f| rsx! { div {}}) });

    let edits = dom.lazy_diff(left, right);
    dbg!(edits);
}

/// Should result in the creation of an anchor (placeholder) and then a replacewith m=5
#[test]
fn empty_fragments_create_many_anchors() {
    let dom = TestDom::new();

    let left = rsx!({ (0..0).map(|f| rsx! { div {}}) });
    let right = rsx!({ (0..5).map(|f| rsx! { div {}}) });

    let edits = dom.lazy_diff(left, right);
    dbg!(edits);
}

/// Should result in the creation of an anchor (placeholder) and then a replacewith
/// Includes child nodes inside the fragment
#[test]
fn empty_fragments_create_anchors_with_many_children() {
    let dom = TestDom::new();

    let left = rsx!({ (0..0).map(|f| rsx! { div {} }) });
    let right = rsx!({
        (0..5).map(|f| {
            rsx! { div { "hello" }}
        })
    });

    let edits = dom.lazy_diff(left, right);
    dbg!(&edits);
    let last_edit = edits.1.edits.last().unwrap();
    assert!(last_edit.is("ReplaceWith"));
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

    let edits = dom.lazy_diff(left, right);
    dbg!(&edits);
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

    let edits = dom.lazy_diff(left, right);
    dbg!(&edits);
    assert!(edits.1.edits.is_empty());
}

/// Should result the creation of more nodes appended after the old last node
#[test]
fn two_fragments_with_differrent_elements_are_differet() {
    let dom = TestDom::new();

    let left = rsx!(
        {(0..2).map(|f| {rsx! { div {  }}})}
        p {}
    );
    let right = rsx!(
        {(0..5).map(|f| {rsx! { h1 {  }}})}
        p {}
    );

    let edits = dom.lazy_diff(left, right);
    dbg!(&edits);
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

    let edits = dom.lazy_diff(left, right);
    dbg!(&edits);
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

    let edits = dom.lazy_diff(left, right);
    dbg!(&edits);
}

// Similar test from above, but with extra child nodes
#[test]
fn two_fragments_with_same_elements_are_differet_shorter() {
    let dom = TestDom::new();

    let left = rsx!(
        {(0..5).map(|f| {rsx! { div {  }}})}
        p {"e"}
    );
    let right = rsx!(
        {(0..2).map(|f| {rsx! { div {  }}})}
        p {"e"}
    );

    let edits = dom.lazy_diff(left, right);
    dbg!(&edits);
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

    let edits = dom.lazy_diff(left, right);
    dbg!(&edits);
}

#[test]
fn fragment_keys() {
    let r = 1;
    let p = rsx! {
        Fragment { key: "asd {r}" }
    };
}

/// Should result in moves, but not removals or additions
#[test]
fn keyed_diffing_out_of_order() {
    let dom = TestDom::new();

    // 0, 1, 2, 3, 4, 5, 6, 7, 8,
    let left = rsx!({
        (0..3).chain(3..6).chain(6..9).map(|f| {
            rsx! { div { key: "{f}"  }}
        })
    });

    // 0, 1, 2, 6, 5, 4, 3, 7, 8, 9
    let right = rsx!({
        (0..3).chain((3..7).rev()).chain(7..10).map(|f| {
            rsx! { div { key: "{f}"  }}
        })
    });

    // LIS: 3, 7, 8,
    let edits = dom.lazy_diff(left, right);
    dbg!(&edits);
}

#[test]
fn controlled_keyed_diffing_out_of_order() {
    let dom = TestDom::new();

    let left = [4, 5, 6, 7];
    let left = rsx!({
        left.iter().map(|f| {
            rsx! { div { key: "{f}" "{f}" }}
        })
    });

    // 0, 1, 2, 6, 5, 4, 3, 7, 8, 9
    let right = [0, 5, 9, 6, 4];
    let right = rsx!({
        right.iter().map(|f| {
            rsx! { div { key: "{f}" "{f}" }}
        })
    });

    // LIS: 3, 7, 8,
    let edits = dom.lazy_diff(left, right);
    dbg!(&edits);
}
