//! This module contains all the code for creating and diffing nodes.
//!
//! For suspense there are three different cases we need to handle:
//! - Creating nodes/scopes without mounting them
//! - Diffing nodes that are not mounted
//! - Mounted nodes that have already been created
//!
//! To support those cases, we lazily create components and only optionally write to the real dom while diffing with Option<&mut impl WriteMutations>

#![allow(clippy::too_many_arguments)]

use crate::{
    ElementId,
    arena::MountId,
    fiber::FiberMode,
    innerlude::{ElementRef, WriteMutations},
    nodes::VNode,
    virtual_dom::VirtualDom,
};

pub(crate) mod anchor;
mod attributes;
mod component;
pub(crate) mod context;
mod iterator;
pub(crate) mod node;

impl VirtualDom {
    pub(crate) fn create_children(
        &mut self,
        to: Option<&mut impl WriteMutations>,
        nodes: &[VNode],
        parent: Option<ElementRef>,
    ) -> usize {
        self.create_children_with_parents(to, nodes, parent, parent)
    }

    pub(crate) fn create_children_with_parents(
        &mut self,
        mut to: Option<&mut impl WriteMutations>,
        nodes: &[VNode],
        render_parent: Option<ElementRef>,
        logical_parent: Option<ElementRef>,
    ) -> usize {
        nodes
            .iter()
            .map(|child| {
                child.create_with_parents(self, render_parent, logical_parent, to.as_deref_mut())
            })
            .sum()
    }

    pub(crate) fn get_mounted_parent(&self, mount: MountId) -> Option<ElementRef> {
        let fibers = self.runtime.fibers.borrow();
        fibers.get(mount.0).and_then(|fiber| fiber.render_parent)
    }

    pub(crate) fn get_mounted_dyn_node(&self, mount: MountId, dyn_node_idx: usize) -> usize {
        let target_id = self.mount_target_id(mount);
        self.runtime.render_targets.borrow()[target_id.0].mounted_fibers[mount.0]
            .as_ref()
            .expect("mounted fiber state should exist")
            .mounted_dynamic_nodes[dyn_node_idx]
    }

    pub(crate) fn set_mounted_dyn_node(&self, mount: MountId, dyn_node_idx: usize, value: usize) {
        let target_id = self.mount_target_id(mount);
        self.runtime.render_targets.borrow_mut()[target_id.0].mounted_fibers[mount.0]
            .as_mut()
            .expect("mounted fiber state should exist")
            .mounted_dynamic_nodes[dyn_node_idx] = value;
    }

    pub(crate) fn get_mounted_dyn_attr(&self, mount: MountId, dyn_attr_idx: usize) -> ElementId {
        let target_id = self.mount_target_id(mount);
        self.runtime.render_targets.borrow()[target_id.0].mounted_fibers[mount.0]
            .as_ref()
            .expect("mounted fiber state should exist")
            .mounted_attributes[dyn_attr_idx]
    }

    pub(crate) fn set_mounted_dyn_attr(
        &self,
        mount: MountId,
        dyn_attr_idx: usize,
        value: ElementId,
    ) {
        let target_id = self.mount_target_id(mount);
        self.runtime.render_targets.borrow_mut()[target_id.0].mounted_fibers[mount.0]
            .as_mut()
            .expect("mounted fiber state should exist")
            .mounted_attributes[dyn_attr_idx] = value;
    }

    pub(crate) fn get_mounted_root_node(&self, mount: MountId, root_idx: usize) -> ElementId {
        let target_id = self.mount_target_id(mount);
        self.runtime.render_targets.borrow()[target_id.0].mounted_fibers[mount.0]
            .as_ref()
            .expect("mounted fiber state should exist")
            .root_ids[root_idx]
    }

    pub(crate) fn set_mounted_root_node(&self, mount: MountId, root_idx: usize, value: ElementId) {
        let target_id = self.mount_target_id(mount);
        self.runtime.render_targets.borrow_mut()[target_id.0].mounted_fibers[mount.0]
            .as_mut()
            .expect("mounted fiber state should exist")
            .root_ids[root_idx] = value;
    }

    pub(crate) fn current_mounted_view(&self, mount: MountId) -> Option<VNode> {
        let fibers = self.runtime.fibers.borrow();
        // Hand out a deep clone so anchor lookups that descend into the
        // returned tree can't observe descendant mount cells being mutated
        // by a sibling diff's `claim_fiber_mount`.
        fibers
            .get(mount.0)
            .map(|fiber| fiber.node.deep_clone_preserving_mounts())
    }

    pub(crate) fn set_fiber_mode(&self, mount: MountId, mode: FiberMode) {
        debug_assert!(mount.mounted(), "set_fiber_mode requires a mounted MountId");
        self.runtime.fibers.borrow_mut()[mount.0].mode = mode;
    }

    pub(crate) fn fiber_should_render(&self, mount: MountId) -> bool {
        // For an unmounted `mount` (`mount.0 == usize::MAX`),
        // `fibers.get(mount.0)` returns `None` and the `is_none_or` predicate
        // short-circuits to `true` — same answer as an explicit early return,
        // so the explicit branch isn't needed.
        self.runtime
            .fibers
            .borrow()
            .get(mount.0)
            .is_none_or(|fiber| fiber.mode == FiberMode::Foreground)
    }

    pub(crate) fn claim_fiber_mount(&self, old: &VNode, new: &VNode) -> MountId {
        let mount = old.mount.take();
        new.mount.set(mount);
        mount
    }

    pub(crate) fn commit_fiber_work(&self, mount: MountId, node: &VNode) {
        // Every caller commits work on a `mount` that's just been claimed via
        // `claim_fiber_mount` or freshly allocated in `create_with_parents` —
        // both produce live `MountId`s, never `PLACEHOLDER`.
        debug_assert!(
            mount.mounted(),
            "commit_fiber_work requires a live MountId"
        );
        // Deep-clone so the committed snapshot owns its own per-vnode
        // `Cell<MountId>` slots. A subsequent diff that calls
        // `claim_fiber_mount` on descendant `old` vnodes would otherwise
        // mutate the shared `Rc<VNodeInner>` here too, and anchor lookups
        // that walk `fiber.node` would see those descendants as unmounted.
        self.runtime.fibers.borrow_mut()[mount.0].node = node.deep_clone_preserving_mounts();
    }

    /// Remove these nodes from the dom
    /// Wont generate mutations for the inner nodes
    fn remove_nodes(&mut self, mut to: Option<&mut impl WriteMutations>, nodes: &[VNode]) {
        for node in nodes.iter().rev() {
            node.remove_node(self, to.as_deref_mut());
        }
    }
}
