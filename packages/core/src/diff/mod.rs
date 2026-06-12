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

    /// Remove these nodes from the dom
    /// Wont generate mutations for the inner nodes
    fn remove_nodes(&mut self, mut to: Option<&mut impl WriteMutations>, nodes: &[VNode]) {
        for node in nodes.iter().rev() {
            node.remove_node(self, to.as_deref_mut());
        }
    }
}
