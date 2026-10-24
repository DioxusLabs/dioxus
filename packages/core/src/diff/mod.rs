//! This module contains all the code for creating and diffing nodes.
//!
//! For suspense there are three different cases we need to handle:
//! - Creating nodes/scopes without mounting them
//! - Diffing nodes that are not mounted
//! - Mounted nodes that have already been created
//!
//! To support those cases, we lazily create components and only optionally write to the real dom
//! while diffing with `Option<&mut (dyn WriteMutations + '_)>`.
//!
//! Core diff invariants:
//! - Every live `MountId` points at exactly one committed `VNode` until that vnode is removed.

use crate::{
    innerlude::{MountId, WriteMutations},
    nodes::VNode,
    virtual_dom::VirtualDom,
};

mod attributes;
mod component;
pub(crate) mod context;
mod iterator;
pub(crate) mod node;
pub(crate) mod placement;
pub(crate) mod template;

pub(crate) struct CreatedVNode {
    pub(crate) nodes: usize,
    pub(crate) mount: MountId,
}

impl VirtualDom {
    /// Create sibling vnodes and report each created mount in input order.
    ///
    /// Invariant: `created_mount` is called exactly once per input vnode after that vnode's mount
    /// state has been materialized.
    pub(crate) fn create_children_with_mounts(
        &mut self,
        mut to: Option<&mut (dyn WriteMutations + '_)>,
        nodes: &[VNode],
        logical_parent: Option<MountId>,
        mut created_mount: impl FnMut(&mut VirtualDom, usize, MountId),
    ) -> usize {
        let mut created = 0;
        for (idx, child) in nodes.iter().enumerate() {
            let child = child.create_mounted(self, logical_parent, to.as_deref_mut());
            created += child.nodes;
            created_mount(self, idx, child.mount);
        }
        created
    }

    /// Remove sibling vnodes in reverse order.
    ///
    /// Invariant: `nodes` and `mounts` describe the same sibling list.
    fn remove_nodes(
        &mut self,
        mut to: Option<&mut (dyn WriteMutations + '_)>,
        nodes: &[VNode],
        mounts: &[MountId],
    ) {
        for (node, mount) in nodes.iter().zip(mounts).rev() {
            node.remove_node(*mount, self, to.as_deref_mut());
        }
    }
}
