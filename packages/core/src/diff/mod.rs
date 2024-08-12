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
    TemplateNode,
};

mod component;
mod iterator;
mod node;

impl VirtualDom {
    pub(crate) fn create_children(
        &mut self,
        mut to: Option<&mut impl WriteMutations>,
        nodes: &[VNode],
        parent: Option<ElementRef>,
    ) -> usize {
        nodes
            .iter()
            .map(|child| child.create(self, parent, to.as_deref_mut()))
            .sum()
    }

    /// Remove these nodes from the dom
    /// Wont generate mutations for the inner nodes
    fn remove_nodes(
        &mut self,
        mut to: Option<&mut impl WriteMutations>,
        nodes: &[VNode],
        replace_with: Option<usize>,
    ) {
        for (i, node) in nodes.iter().rev().enumerate() {
            let last_node = i == nodes.len() - 1;
            node.remove_node(self, to.as_deref_mut(), replace_with.filter(|_| last_node));
        }
    }
}

/// We can apply various optimizations to dynamic nodes that are the single child of their parent.
///
/// IE
///  - for text - we can use SetTextContent
///  - for clearing children we can use RemoveChildren
///  - for appending children we can use AppendChildren
#[allow(dead_code)]
fn is_dyn_node_only_child(node: &VNode, idx: usize) -> bool {
    let template = node.template;
    let path = template.node_paths[idx];

    // use a loop to index every static node's children until the path has run out
    // only break if the last path index is a dynamic node
    let mut static_node = &template.roots[path[0] as usize];

    for i in 1..path.len() - 1 {
        match static_node {
            TemplateNode::Element { children, .. } => static_node = &children[path[i] as usize],
            _ => return false,
        }
    }

    match static_node {
        TemplateNode::Element { children, .. } => children.len() == 1,
        _ => false,
    }
}
