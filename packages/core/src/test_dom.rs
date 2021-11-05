//! A DOM for testing - both internal and external code.
use bumpalo::Bump;

use crate::innerlude::*;
use crate::nodes::IntoVNode;

pub struct TestDom {
    bump: Bump,
    // scheduler: Scheduler,
}

impl TestDom {
    pub fn new() -> TestDom {
        let bump = Bump::new();
        let (sender, receiver) = futures_channel::mpsc::unbounded::<SchedulerMsg>();
        todo!()
        // let scheduler = Scheduler::new(sender, receiver, 10, 100);
        // TestDom { bump, scheduler }
    }

    pub fn new_factory(&self) -> NodeFactory {
        NodeFactory::new(&self.bump)
    }

    pub fn render_direct<'a>(&'a self, lazy_nodes: Option<LazyNodes<'a, '_>>) -> VNode<'a> {
        lazy_nodes.into_vnode(NodeFactory::new(&self.bump))
    }

    pub fn render<'a>(&'a self, lazy_nodes: Option<LazyNodes<'a, '_>>) -> &'a VNode<'a> {
        self.bump
            .alloc(lazy_nodes.into_vnode(NodeFactory::new(&self.bump)))
    }

    pub fn diff<'a>(&'a self, old: &'a VNode<'a>, new: &'a VNode<'a>) -> Mutations<'a> {
        let mutations = Mutations::new();
        let mut machine = DiffMachine::new(mutations);
        machine.stack.push(DiffInstruction::Diff { new, old });
        machine.mutations
    }

    pub fn create<'a>(&'a self, left: Option<LazyNodes<'a, '_>>) -> Mutations<'a> {
        let old = self.bump.alloc(self.render_direct(left));

        let mut machine = DiffMachine::new(Mutations::new());

        machine.stack.create_node(old, MountType::Append);

        machine.work(&mut || false);

        machine.mutations
    }

    pub fn lazy_diff<'a>(
        &'a self,
        left: Option<LazyNodes<'a, '_>>,
        right: Option<LazyNodes<'a, '_>>,
    ) -> (Mutations<'a>, Mutations<'a>) {
        let (old, new) = (self.render(left), self.render(right));

        let mut machine = DiffMachine::new(Mutations::new());

        machine.stack.create_node(old, MountType::Append);

        machine.work(|| false);
        let create_edits = machine.mutations;

        let mut machine = DiffMachine::new(Mutations::new());

        machine.stack.push(DiffInstruction::Diff { old, new });

        machine.work(&mut || false);

        let edits = machine.mutations;

        (create_edits, edits)
    }
}

impl Default for TestDom {
    fn default() -> Self {
        Self::new()
    }
}
