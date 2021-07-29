use bumpalo::Bump;

use anyhow::{Context, Result};
use dioxus::{
    arena::SharedResources,
    diff::{CreateMeta, DiffMachine},
    prelude::*,
    util::DebugDom,
    DomEdit,
};
use dioxus_core as dioxus;
use dioxus_html as dioxus_elements;

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

    fn diff<'a>(&'a self, old: &'a VNode<'a>, new: &'a VNode<'a>) -> Vec<DomEdit<'a>> {
        let mut edits = Vec::new();
        let dom = DebugDom::new();
        let mut machine = DiffMachine::new_headless(&mut edits, &dom, &self.resources);
        machine.diff_node(old, new);
        edits
    }

    fn create<'a, F1>(&'a self, left: LazyNodes<'a, F1>) -> (CreateMeta, Vec<DomEdit<'a>>)
    where
        F1: FnOnce(NodeFactory<'a>) -> VNode<'a>,
    {
        let old = self.bump.alloc(self.render(left));
        let mut edits = Vec::new();
        let dom = DebugDom::new();

        let mut machine = DiffMachine::new_headless(&mut edits, &dom, &self.resources);
        let meta = machine.create_vnode(old);
        (meta, edits)
    }

    fn lazy_diff<'a, F1, F2>(
        &'a self,
        left: LazyNodes<'a, F1>,
        right: LazyNodes<'a, F2>,
    ) -> Vec<DomEdit<'a>>
    where
        F1: FnOnce(NodeFactory<'a>) -> VNode<'a>,
        F2: FnOnce(NodeFactory<'a>) -> VNode<'a>,
    {
        let old = self.bump.alloc(self.render(left));

        let new = self.bump.alloc(self.render(right));

        let mut edits = Vec::new();
        let dom = DebugDom::new();

        let mut machine = DiffMachine::new_headless(&mut edits, &dom, &self.resources);
        machine.create_vnode(old);
        edits.clear();

        let mut machine = DiffMachine::new_headless(&mut edits, &dom, &self.resources);
        machine.diff_node(old, new);
        edits
    }
}

#[test]
fn diffing_works() {}

#[test]
fn html_and_rsx_generate_the_same_output() {
    let dom = TestDom::new();

    let edits = dom.lazy_diff(
        rsx! ( div { "Hello world" } ),
        rsx! ( div { "Goodbye world" } ),
    );
    dbg!(edits);
}

#[test]
fn fragments_create_properly() {
    let dom = TestDom::new();
    let (meta, edits) = dom.create(rsx! {
        div { "Hello a" }
        div { "Hello b" }
        div { "Hello c" }
    });
    assert!(&edits[0].is("CreateElement"));
    assert!(&edits[3].is("CreateElement"));
    assert!(&edits[6].is("CreateElement"));

    assert_eq!(meta.added_to_stack, 3);
    dbg!(edits);
}
