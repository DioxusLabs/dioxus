//! This module contains all the code for creating and diffing nodes.
//!
//! For suspense there are three different cases we need to handle:
//! - Creating nodes/scopes without mounting them
//! - Diffing nodes that are not mounted
//! - Mounted nodes that have already been created
//!
//! To support those cases, we separate node creation into initialization and mounting. We also keep track of if we should be adding mutations or not.

#![allow(clippy::too_many_arguments)]

use crate::{
    arena::ElementId,
    innerlude::{ElementRef, MountId, WriteMutations},
    nodes::VNode,
    virtual_dom::VirtualDom,
    Template, TemplateNode,
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

    /// Simply replace a placeholder with a list of nodes
    fn replace_placeholder(
        &mut self,
        mut to: Option<&mut impl WriteMutations>,
        placeholder_id: ElementId,
        r: &[VNode],
        parent: Option<ElementRef>,
    ) {
        let m = self.create_children(to.as_deref_mut(), r, parent);
        if let Some(to) = to {
            to.replace_node_with(placeholder_id, m);
            self.reclaim(placeholder_id);
        }
    }

    fn nodes_to_placeholder(
        &mut self,
        mut to: Option<&mut impl WriteMutations>,
        mount: MountId,
        dyn_node_idx: usize,
        old_nodes: &[VNode],
    ) {
        // Create the placeholder first, ensuring we get a dedicated ID for the placeholder
        let placeholder = self.next_element();

        // Set the id of the placeholder
        self.mounts[mount.0].mounted_dynamic_nodes[dyn_node_idx] = placeholder.0;

        if let Some(to) = to.as_deref_mut() {
            to.create_placeholder(placeholder);
        }

        self.replace_nodes(to, old_nodes, 1);
    }

    /// Replace many nodes with a number of nodes on the stack
    fn replace_nodes(&mut self, to: Option<&mut impl WriteMutations>, nodes: &[VNode], m: usize) {
        debug_assert!(
            !nodes.is_empty(),
            "replace_nodes must have at least one node"
        );

        // We want to optimize the replace case to use one less mutation if possible
        // Instead of *just* removing it, we can use the replace mutation
        self.remove_nodes(to, nodes, Some(m));
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

    /// Insert a new template into the VirtualDom's template registry
    // used in conditional compilation
    #[allow(unused_mut)]
    pub(crate) fn register_template(
        &mut self,
        to: &mut impl WriteMutations,
        mut template: Template,
    ) {
        // In debug mode, we check the more complete hashmap by byte index
        #[cfg(debug_assertions)]
        {
            let (path, byte_index) = template.name.rsplit_once(':').unwrap();

            let byte_index = byte_index.parse::<usize>().unwrap();
            let mut entry = self.templates.entry(path);
            // If we've already seen this template, just return
            if let std::collections::hash_map::Entry::Occupied(occupied) = &entry {
                if occupied.get().contains_key(&byte_index) {
                    return;
                }
            }

            // Otherwise, insert it and register it
            entry.or_default().insert(byte_index, template);
        }

        // In release mode, everything is built into the &'static str
        #[cfg(not(debug_assertions))]
        if !self.templates.insert(template.name) {
            return;
        }

        // If it's all dynamic nodes, then we don't need to register it
        if !template.is_completely_dynamic() {
            to.register_template(template)
        }
    }

    #[cfg(debug_assertions)]
    /// Insert a new template into the VirtualDom's template registry
    pub(crate) fn register_template_first_byte_index(&mut self, mut template: Template) {
        // First, make sure we mark the template as seen, regardless if we process it
        let (path, _) = template.name.rsplit_once(':').unwrap();
        if let Some((_, old_template)) = self
            .templates
            .entry(path)
            .or_default()
            .iter_mut()
            .min_by_key(|(byte_index, _)| **byte_index)
        {
            // the byte index of the hot reloaded template could be different
            template.name = old_template.name;
            *old_template = template;
        } else {
            // This is a template without any current instances
            self.templates
                .entry(path)
                .or_default()
                .insert(usize::MAX, template);
        }

        // If it's all dynamic nodes, then we don't need to register it
        if !template.is_completely_dynamic() {
            self.queued_templates.push(template);
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
    let template = node.template.get();
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
