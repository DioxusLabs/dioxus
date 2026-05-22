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
        fibers.get(mount.0).map(|fiber| fiber.node.clone())
    }

    pub(crate) fn set_fiber_mode(&self, mount: MountId, mode: FiberMode) {
        if mount.mounted()
            && let Some(fiber) = self.runtime.fibers.borrow_mut().get_mut(mount.0)
        {
            fiber.mode = mode;
        }
    }

    pub(crate) fn fiber_should_render(&self, mount: MountId) -> bool {
        if !mount.mounted() {
            return true;
        }
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
        if mount.mounted()
            && let Some(fiber) = self.runtime.fibers.borrow_mut().get_mut(mount.0)
        {
            fiber.node = node.clone();
        }
    }

    /// Remove these nodes from the dom
    /// Wont generate mutations for the inner nodes
    fn remove_nodes(&mut self, mut to: Option<&mut impl WriteMutations>, nodes: &[VNode]) {
        for node in nodes.iter().rev() {
            node.remove_node(self, to.as_deref_mut());
        }
    }
}
