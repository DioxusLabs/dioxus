//! This module contains all the code for creating and diffing nodes.
//!
//! For suspense there are three different cases we need to handle:
//! - Creating nodes/scopes without mounting them
//! - Diffing nodes that are not mounted
//! - Mounted nodes that have already been created
//!
//! To support those cases, we lazily create components and only optionally write to the real dom while diffing with Option<&mut dyn WriteMutations>

#![allow(clippy::too_many_arguments)]

use crate::{
    DynamicNode,
    innerlude::{MountId, MountRef, WriteMutations},
    mutations::reborrow_writer,
    nodes::VNode,
    virtual_dom::VirtualDom,
};

mod attributes;
mod component;
pub(crate) mod context;
mod iterator;
pub(crate) mod node;
pub(crate) mod placement;
mod template;

#[derive(Debug)]
pub(crate) struct CreatedVNode {
    pub(crate) nodes: usize,
    pub(crate) mount: MountId,
}

#[derive(Debug)]
pub(crate) struct CreatedNodes {
    pub(crate) nodes: usize,
    pub(crate) mounts: Vec<MountId>,
}

impl VirtualDom {
    pub(crate) fn create_children_with_parents(
        &mut self,
        mut to: Option<&mut dyn WriteMutations>,
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
            let child = child.create_with_parents(
                self,
                render_parent,
                logical_parent,
                reborrow_writer(&mut to),
            );
            created.nodes += child.nodes;
            created.mounts.push(child.mount);
        }
        created
    }

    fn reserve_fragment_children(&mut self, nodes: &[VNode]) {
        if nodes.is_empty() {
            return;
        }

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

    /// Remove these nodes from the dom
    /// Wont generate mutations for the inner nodes
    fn remove_nodes(
        &mut self,
        mut to: Option<&mut dyn WriteMutations>,
        nodes: &[VNode],
        mounts: &[MountId],
    ) {
        for (node, mount) in nodes.iter().zip(mounts).rev() {
            node.remove_node(*mount, self, reborrow_writer(&mut to));
        }
    }
}

fn root_component_count(node: &VNode) -> usize {
    node.template
        .root_slots()
        .filter_map(|(_, _, anchor)| anchor)
        .map(|anchor| {
            anchor
                .values()
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
