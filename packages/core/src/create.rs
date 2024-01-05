use crate::innerlude::{ElementPath, ElementRef, VComponent, VPlaceholder, VText, WriteMutations};
use crate::nodes::VNode;
use crate::nodes::{DynamicNode, TemplateNode};
use crate::virtual_dom::VirtualDom;
use crate::{AttributeValue, ElementId, RenderReturn, ScopeId, Template};
use std::cell::Cell;
use std::iter::Peekable;
use TemplateNode::*;

#[cfg(debug_assertions)]
fn sort_bfs(paths: &[&'static [u8]]) -> Vec<(usize, &'static [u8])> {
    let mut with_indecies = paths.iter().copied().enumerate().collect::<Vec<_>>();
    with_indecies.sort_unstable_by(|(_, a), (_, b)| {
        let mut a = a.iter();
        let mut b = b.iter();
        loop {
            match (a.next(), b.next()) {
                (Some(a), Some(b)) => {
                    if a != b {
                        return a.cmp(b);
                    }
                }
                // The shorter path goes first
                (None, Some(_)) => return std::cmp::Ordering::Less,
                (Some(_), None) => return std::cmp::Ordering::Greater,
                (None, None) => return std::cmp::Ordering::Equal,
            }
        }
    });
    with_indecies
}

#[test]
#[cfg(debug_assertions)]
fn sorting() {
    let r: [(usize, &[u8]); 5] = [
        (0, &[0, 1]),
        (1, &[0, 2]),
        (2, &[1, 0]),
        (3, &[1, 0, 1]),
        (4, &[1, 2]),
    ];
    assert_eq!(
        sort_bfs(&[&[0, 1,], &[0, 2,], &[1, 0,], &[1, 0, 1,], &[1, 2,],]),
        r
    );
    let r: [(usize, &[u8]); 6] = [
        (0, &[0]),
        (1, &[0, 1]),
        (2, &[0, 1, 2]),
        (3, &[1]),
        (4, &[1, 2]),
        (5, &[2]),
    ];
    assert_eq!(
        sort_bfs(&[&[0], &[0, 1], &[0, 1, 2], &[1], &[1, 2], &[2],]),
        r
    );
}

impl VirtualDom {
    /// Create a new template [`VNode`] and write it to the [`Mutations`] buffer.
    ///
    /// This method pushes the ScopeID to the internal scopestack and returns the number of nodes created.
    pub(crate) fn create_scope(
        &mut self,
        scope: ScopeId,
        template: &VNode,
        to: &mut impl WriteMutations,
    ) -> usize {
        self.runtime.scope_stack.borrow_mut().push(scope);
        let nodes = self.create(template, to);
        self.runtime.scope_stack.borrow_mut().pop();
        nodes
    }

    /// Create this template and write its mutations
    pub(crate) fn create(&mut self, node: &VNode, to: &mut impl WriteMutations) -> usize {
        // check for a overriden template
        #[cfg(debug_assertions)]
        {
            let (path, byte_index) = node.template.get().name.rsplit_once(':').unwrap();
            if let Some(template) = self
                .templates
                .get(path)
                .and_then(|map| map.get(&byte_index.parse().unwrap()))
            {
                node.template.set(*template);
            }
        }

        // Initialize the root nodes slice
        {
            let mut nodes_mut = node.root_ids.borrow_mut();
            let len = node.template.get().roots.len();
            nodes_mut.resize(len, ElementId::default());
        };

        // The best renderers will have templates prehydrated and registered
        // Just in case, let's create the template using instructions anyways
        self.register_template(node.template.get(), to);

        // Walk the roots, creating nodes and assigning IDs
        // nodes in an iterator of ((dynamic_node_index, sorted_index), path)
        // todo: adjust dynamic nodes to be in the order of roots and then leaves (ie BFS)
        #[cfg(not(debug_assertions))]
        let (mut attrs, mut nodes) = (
            node.template
                .get()
                .attr_paths
                .iter()
                .copied()
                .enumerate()
                .peekable(),
            node.template
                .get()
                .node_paths
                .iter()
                .copied()
                .enumerate()
                .map(|(i, path)| ((i, i), path))
                .peekable(),
        );
        // If this is a debug build, we need to check that the paths are in the correct order because hot reloading can cause scrambled states

        #[cfg(debug_assertions)]
        let (attrs_sorted, nodes_sorted) = {
            (
                sort_bfs(node.template.get().attr_paths),
                sort_bfs(node.template.get().node_paths),
            )
        };
        #[cfg(debug_assertions)]
        let (mut attrs, mut nodes) = {
            (
                attrs_sorted.into_iter().peekable(),
                nodes_sorted
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(i, (id, path))| ((id, i), path))
                    .peekable(),
            )
        };

        node.template
            .get()
            .roots
            .iter()
            .enumerate()
            .map(|(idx, root)| match root {
                DynamicText { id } | Dynamic { id } => {
                    nodes.next().unwrap();
                    self.write_dynamic_root(node, *id, to)
                }
                Element { .. } => {
                    #[cfg(not(debug_assertions))]
                    let id = self.write_element_root(node, idx, &mut attrs, &mut nodes, &[]);
                    #[cfg(debug_assertions)]
                    let id = self.write_element_root(
                        node,
                        idx,
                        &mut attrs,
                        &mut nodes,
                        &nodes_sorted,
                        to,
                    );
                    id
                }
                Text { .. } => self.write_static_text_root(node, idx, to),
            })
            .sum()
    }

    fn write_static_text_root(
        &mut self,
        node: &VNode,
        idx: usize,
        to: &mut impl WriteMutations,
    ) -> usize {
        // Simply just load the template root, no modifications needed
        self.load_template_root(node, idx, to);

        // Text producs just one node on the stack
        1
    }

    fn write_dynamic_root(
        &mut self,
        template: &VNode,
        idx: usize,
        to: &mut impl WriteMutations,
    ) -> usize {
        use DynamicNode::*;
        match &template.dynamic_nodes[idx] {
            node @ Component { .. } | node @ Fragment(_) => {
                let template_ref = ElementRef {
                    path: ElementPath {
                        path: template.template.get().node_paths[idx],
                    },
                    element: template.clone(),
                    scope: self.runtime.current_scope_id().unwrap_or(ScopeId(0)),
                };
                self.create_dynamic_node(&template_ref, node, to)
            }
            Placeholder(VPlaceholder { id, parent }) => {
                let template_ref = ElementRef {
                    path: ElementPath {
                        path: template.template.get().node_paths[idx],
                    },
                    element: template.clone(),
                    scope: self.runtime.current_scope_id().unwrap_or(ScopeId(0)),
                };
                *parent.borrow_mut() = Some(template_ref);
                let id = self.set_slot(id);
                to.create_placeholder(id);
                1
            }
            Text(VText { id, value }) => {
                let id = self.set_slot(id);
                to.create_text_node(value, id);
                1
            }
        }
    }

    /// We write all the descendent data for this element
    ///
    /// Elements can contain other nodes - and those nodes can be dynamic or static
    ///
    /// We want to make sure we write these nodes while on top of the root
    fn write_element_root(
        &mut self,
        template: &VNode,
        root_idx: usize,
        dynamic_attrs: &mut Peekable<impl Iterator<Item = (usize, &'static [u8])>>,
        dynamic_nodes_iter: &mut Peekable<impl Iterator<Item = ((usize, usize), &'static [u8])>>,
        dynamic_nodes: &[(usize, &'static [u8])],
        to: &mut impl WriteMutations,
    ) -> usize {
        // Load the template root and get the ID for the node on the stack
        let root_on_stack = self.load_template_root(template, root_idx, to);

        // Write all the attributes below this root
        self.write_attrs_on_root(dynamic_attrs, root_idx as u8, root_on_stack, template, to);

        // Load in all of the placeholder or dynamic content under this root too
        self.load_placeholders(
            dynamic_nodes_iter,
            dynamic_nodes,
            root_idx as u8,
            template,
            to,
        );

        1
    }

    /// Load all of the placeholder nodes for descendents of this root node
    ///
    /// ```rust, ignore
    /// rsx! {
    ///     div {
    ///         // This is a placeholder
    ///         some_value,
    ///
    ///         // Load this too
    ///         "{some_text}"
    ///     }
    /// }
    /// ```
    #[allow(unused)]
    fn load_placeholders(
        &mut self,
        dynamic_nodes_iter: &mut Peekable<impl Iterator<Item = ((usize, usize), &'static [u8])>>,
        dynamic_nodes: &[(usize, &'static [u8])],
        root_idx: u8,
        template: &VNode,
        to: &mut impl WriteMutations,
    ) {
        let (start, end) = match collect_dyn_node_range(dynamic_nodes_iter, root_idx) {
            Some((a, b)) => (a, b),
            None => return,
        };

        // If hot reloading is enabled, we need to map the sorted index to the original index of the dynamic node. If it is disabled, we can just use the sorted index
        #[cfg(not(debug_assertions))]
        let reversed_iter = (start..=end).rev();
        #[cfg(debug_assertions)]
        let reversed_iter = (start..=end)
            .rev()
            .map(|sorted_index| dynamic_nodes[sorted_index].0);

        for idx in reversed_iter {
            let boundary_ref = ElementRef {
                path: ElementPath {
                    path: template.template.get().node_paths[idx],
                },
                element: template.clone(),
                scope: self.runtime.current_scope_id().unwrap_or(ScopeId(0)),
            };
            let m = self.create_dynamic_node(&boundary_ref, &template.dynamic_nodes[idx], to);
            if m > 0 {
                // The path is one shorter because the top node is the root
                let path = &template.template.get().node_paths[idx][1..];
                to.replace_placeholder_with_nodes(path, m);
            }
        }
    }

    fn write_attrs_on_root(
        &mut self,
        attrs: &mut Peekable<impl Iterator<Item = (usize, &'static [u8])>>,
        root_idx: u8,
        root: ElementId,
        node: &VNode,
        to: &mut impl WriteMutations,
    ) {
        while let Some((mut attr_id, path)) =
            attrs.next_if(|(_, p)| p.first().copied() == Some(root_idx))
        {
            let id = self.assign_static_node_as_dynamic(path, root, to);

            loop {
                self.write_attribute(node, attr_id, &node.dynamic_attrs[attr_id], id, to);

                // Only push the dynamic attributes forward if they match the current path (same element)
                match attrs.next_if(|(_, p)| *p == path) {
                    Some((next_attr_id, _)) => attr_id = next_attr_id,
                    None => break,
                }
            }
        }
    }

    fn write_attribute(
        &mut self,
        template: &VNode,
        idx: usize,
        attribute: &crate::Attribute,
        id: ElementId,
        to: &mut impl WriteMutations,
    ) {
        // Make sure we set the attribute's associated id
        attribute.mounted_element.set(id);

        // Safety: we promise not to re-alias this text later on after committing it to the mutation
        let unbounded_name: &str = attribute.name;

        match &attribute.value {
            AttributeValue::Listener(_) => {
                let path = &template.template.get().attr_paths[idx];
                let element_ref = ElementRef {
                    path: ElementPath { path },
                    element: template.clone(),
                    scope: self.runtime.current_scope_id().unwrap_or(ScopeId(0)),
                };
                self.elements[id.0] = Some(element_ref);
                to.create_event_listener(&unbounded_name[2..], id);
            }
            _ => {
                let unbounded_value = &attribute.value;

                to.set_attribute(unbounded_name, attribute.namespace, unbounded_value, id);
            }
        }
    }

    fn load_template_root(
        &mut self,
        template: &VNode,
        root_idx: usize,
        to: &mut impl WriteMutations,
    ) -> ElementId {
        // Get an ID for this root since it's a real root
        let this_id = self.next_element();
        template.root_ids.borrow_mut()[root_idx] = this_id;

        to.load_template(template.template.get().name, root_idx, this_id);

        this_id
    }

    /// We have some dynamic attributes attached to a some node
    ///
    /// That node needs to be loaded at runtime, so we need to give it an ID
    ///
    /// If the node in question is on the stack, we just return that ID
    ///
    /// If the node is not on the stack, we create a new ID for it and assign it
    fn assign_static_node_as_dynamic(
        &mut self,
        path: &'static [u8],
        this_id: ElementId,
        to: &mut impl WriteMutations,
    ) -> ElementId {
        if path.len() == 1 {
            return this_id;
        }

        // if attribute is on a root node, then we've already created the element
        // Else, it's deep in the template and we should create a new id for it
        let id = self.next_element();

        to.assign_node_id(&path[1..], id);

        id
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

    /// Insert a new template into the VirtualDom's template registry
    // used in conditional compilation
    #[allow(unused_mut)]
    pub(crate) fn register_template(
        &mut self,
        mut template: Template,
        to: &mut impl WriteMutations,
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

    pub(crate) fn create_dynamic_node(
        &mut self,
        parent: &ElementRef,
        node: &DynamicNode,
        to: &mut impl WriteMutations,
    ) -> usize {
        use DynamicNode::*;
        match node {
            Text(text) => self.create_dynamic_text(parent, text, to),
            Placeholder(place) => self.create_placeholder(place, parent, to),
            Component(component) => self.create_component_node(Some(parent), component, to),
            Fragment(frag) => self.create_children(frag, Some(parent), to),
        }
    }

    fn create_dynamic_text(
        &mut self,
        parent: &ElementRef,
        text: &VText,
        to: &mut impl WriteMutations,
    ) -> usize {
        // Allocate a dynamic element reference for this text node
        let new_id = self.next_element();

        // Make sure the text node is assigned to the correct element
        text.id.set(Some(new_id));

        // Add the mutation to the list
        to.hydrate_text_node(&parent.path.path[1..], &text.value, new_id);

        // Since we're hydrating an existing node, we don't create any new nodes
        0
    }

    pub(crate) fn create_placeholder(
        &mut self,
        placeholder: &VPlaceholder,
        parent: &ElementRef,
        to: &mut impl WriteMutations,
    ) -> usize {
        // Allocate a dynamic element reference for this text node
        let id = self.next_element();

        // Make sure the text node is assigned to the correct element
        placeholder.id.set(Some(id));

        // Assign the placeholder's parent
        *placeholder.parent.borrow_mut() = Some(parent.clone());

        // Assign the ID to the existing node in the template
        to.assign_node_id(&parent.path.path[1..], id);

        // Since the placeholder is already in the DOM, we don't create any new nodes
        0
    }

    pub(super) fn create_component_node(
        &mut self,
        parent: Option<&ElementRef>,
        component: &VComponent,
        to: &mut impl WriteMutations,
    ) -> usize {
        use RenderReturn::*;

        // Load up a ScopeId for this vcomponent
        let scope = self.load_scope_from_vcomponent(component, to);

        component.scope.set(Some(scope));

        let new = self.run_scope(scope);

        self.scopes[scope.0].last_rendered_node = Some(new.clone());

        match &new {
            // Create the component's root element
            Ready(t) => {
                self.assign_boundary_ref(parent, t);
                self.create_scope(scope, t, to)
            }
            Aborted(t) => self.mount_aborted(t, parent, to),
        }
    }

    /// Load a scope from a vcomponent. If the scope id doesn't exist, that means the component is currently "live"
    fn load_scope_from_vcomponent(
        &mut self,
        component: &VComponent,
        _to: &mut impl WriteMutations,
    ) -> ScopeId {
        component.scope.get().unwrap_or_else(|| {
            self.new_scope(component.props.clone(), component.name)
                .context()
                .id
        })
    }

    fn mount_aborted(
        &mut self,
        placeholder: &VPlaceholder,
        parent: Option<&ElementRef>,
        to: &mut impl WriteMutations,
    ) -> usize {
        let id = self.next_element();
        to.create_placeholder(id);
        placeholder.id.set(Some(id));
        *placeholder.parent.borrow_mut() = parent.cloned();

        1
    }

    fn set_slot(&mut self, slot: &Cell<Option<ElementId>>) -> ElementId {
        let id = self.next_element();
        slot.set(Some(id));
        id
    }
}

fn collect_dyn_node_range(
    dynamic_nodes: &mut Peekable<impl Iterator<Item = ((usize, usize), &'static [u8])>>,
    root_idx: u8,
) -> Option<(usize, usize)> {
    let start = match dynamic_nodes.peek() {
        Some(((_, idx), [first, ..])) if *first == root_idx => *idx,
        _ => return None,
    };

    let mut end = start;

    while let Some(((_, idx), p)) =
        dynamic_nodes.next_if(|(_, p)| matches!(p, [idx, ..] if *idx == root_idx))
    {
        if p.len() == 1 {
            continue;
        }

        end = idx;
    }

    Some((start, end))
}
