#![allow(clippy::too_many_arguments)]

use crate::{
    arena::ElementId,
    innerlude::{ElementRef, MountId, WriteMutations},
    nodes::VNode,
    scopes::ScopeId,
    virtual_dom::VirtualDom,
    Template, TemplateNode,
};

mod component;
mod iterator;
mod node;

impl VirtualDom {
    pub(crate) fn create_children<'a>(
        &mut self,
        to: &mut impl WriteMutations,
        nodes: impl IntoIterator<Item = &'a VNode>,
        parent: Option<ElementRef>,
    ) -> usize {
        nodes
            .into_iter()
            .map(|child| child.create(self, to, parent))
            .sum()
    }

    /// Simply replace a placeholder with a list of nodes
    fn replace_placeholder<'a>(
        &mut self,
        to: &mut impl WriteMutations,
        placeholder_id: ElementId,
        r: impl IntoIterator<Item = &'a VNode>,
        parent: Option<ElementRef>,
    ) {
        let m = self.create_children(to, r, parent);
        to.replace_node_with(placeholder_id, m);
        self.reclaim(placeholder_id);
    }

    fn nodes_to_placeholder(
        &mut self,
        to: &mut impl WriteMutations,
        mount: MountId,
        dyn_node_idx: usize,
        old_nodes: &[VNode],
    ) {
        // Create the placeholder first, ensuring we get a dedicated ID for the placeholder
        let placeholder = self.next_element();

        // Set the id of the placeholder
        self.mounts[mount.0].mounted_dynamic_nodes[dyn_node_idx] = placeholder.0;

        to.create_placeholder(placeholder);

        self.replace_nodes(to, old_nodes, 1);
    }

    /// Replace many nodes with a number of nodes on the stack
    fn replace_nodes(&mut self, to: &mut impl WriteMutations, nodes: &[VNode], m: usize) {
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
        to: &mut impl WriteMutations,
        nodes: &[VNode],
        replace_with: Option<usize>,
    ) {
        for (i, node) in nodes.iter().rev().enumerate() {
            let last_node = i == nodes.len() - 1;
            node.remove_node(self, to, replace_with.filter(|_| last_node), true);
        }
    }

    pub(crate) fn remove_component_node(
        &mut self,
        to: &mut impl WriteMutations,
        scope: ScopeId,
        replace_with: Option<usize>,
        gen_muts: bool,
    ) {
        // Remove the component from the dom
        if let Some(node) = self.scopes[scope.0].last_rendered_node.take() {
            node.remove_node(self, to, replace_with, gen_muts)
        };

        // Now drop all the resources
        self.drop_scope(scope);
    }

    /// Insert a new template into the VirtualDom's template registry
    // used in conditional compilation
    #[allow(unused_mut)]
    pub(crate) fn register_template(
        &mut self,
        to: &mut impl WriteMutations,
        mut template: Template,
    ) {
        let (path, byte_index) = template.name.rsplit_once(':').unwrap();

        let byte_index = byte_index.parse::<usize>().unwrap();
        // First, check if we've already seen this template
        if self
            .templates
            .get(&path)
            .filter(|set| set.contains_key(&byte_index))
            .is_none()
        {
            // if hot reloading is enabled, then we need to check for a template that has overriten this one
            #[cfg(debug_assertions)]
            if let Some(mut new_template) = self
                .templates
                .get_mut(path)
                .and_then(|map| map.remove(&usize::MAX))
            {
                // the byte index of the hot reloaded template could be different
                new_template.name = template.name;
                template = new_template;
            }

            self.templates
                .entry(path)
                .or_default()
                .insert(byte_index, template);

            // If it's all dynamic nodes, then we don't need to register it
            if !template.is_completely_dynamic() {
                to.register_template(template)
            }
        }
    }

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
