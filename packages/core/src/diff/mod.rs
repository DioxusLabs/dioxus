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
//! - A vnode commit is atomic from the parent fragment's point of view: child mount lists are
//!   replaced only after the whole fragment diff has chosen placement anchors.
//! - `None` writers mean "maintain mount/component state without renderer mutations", not "the
//!   mount graph may be incomplete".
//! - Internal diff code that needs a fragment's child mounts must use exact accessors; permissive
//!   mounted-node queries are reserved for public inspection/event lookup paths.

#![allow(clippy::too_many_arguments)]

use crate::{
    DynamicNode,
    innerlude::{MountId, MountRef, WriteMutations},
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

pub(crate) struct CreatedNodes {
    pub(crate) nodes: usize,
    pub(crate) mounts: Vec<MountId>,
}

impl VirtualDom {
    /// Create sibling vnodes under one render/logical parent.
    ///
    /// Invariant: the returned `mounts` has exactly one mount per input vnode, in input order. If
    /// `to` is `None`, mount/component state is still fully materialized while renderer nodes are
    /// not emitted.
    pub(crate) fn create_children_with_parents(
        &mut self,
        mut to: Option<&mut (dyn WriteMutations + '_)>,
        nodes: &[VNode],
        render_parent: Option<MountRef>,
        logical_parent: Option<MountRef>,
    ) -> CreatedNodes {
        self.reserve_fragment_children(nodes);

        let mut created = CreatedNodes {
            nodes: 0,
            mounts: Vec::with_capacity(nodes.len()),
        };
        for child in nodes {
            let child =
                child.create_with_parents(self, render_parent, logical_parent, to.as_deref_mut());
            created.nodes += child.nodes;
            created.mounts.push(child.mount);
        }
        created
    }

    /// Reserve enough mount/scope storage for a fragment creation.
    ///
    /// Invariant: this only affects allocation capacity; it does not allocate mount ids or mutate
    /// the committed mount graph.
    fn reserve_fragment_children(&mut self, nodes: &[VNode]) {
        self.runtime.mounts.borrow_mut().reserve(nodes.len());

        let root_components = nodes.iter().map(root_component_count).sum::<usize>();
        if root_components == 0 {
            return;
        }

        self.scopes.reserve(root_components);
        self.runtime
            .scope_states
            .borrow_mut()
            .reserve(root_components);
    }

    /// Remove sibling vnodes in reverse order.
    ///
    /// Invariant: `nodes` and `mounts` describe the same sibling list. Inner removals do not emit
    /// their own mutations because the parent-level removal owns those renderer operations.
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

/// Count root-level component dynamic nodes for scope storage reservation.
///
/// Invariant: this is only a capacity hint. Non-root component scopes are allocated lazily when
/// their owning template root is materialized.
fn root_component_count(node: &VNode) -> usize {
    node.dynamic_nodes()
        .filter(|group| group.is_root_level())
        .map(|group| {
            group
                .ids()
                .filter(|idx| {
                    matches!(
                        node.dynamic_values[*idx].as_node(),
                        Some(DynamicNode::Component(_))
                    )
                })
                .count()
        })
        .sum()
}
