use crate::any_props::AnyProps;
use crate::innerlude::{BorrowedAttributeValue, VComponent, VPlaceholder, VText};
use crate::mutations::Mutation;
use crate::mutations::Mutation::*;
use crate::nodes::VNode;
use crate::nodes::{DynamicNode, TemplateNode};
use crate::virtual_dom::VirtualDom;
use crate::{AttributeValue, ElementId, RenderReturn, ScopeId, SuspenseContext, Template};
use std::cell::Cell;
use std::iter::Peekable;
use std::rc::Rc;
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

impl<'b> VirtualDom {
    /// Create a new template [`VNode`] and write it to the [`Mutations`] buffer.
    ///
    /// This method pushes the ScopeID to the internal scopestack and returns the number of nodes created.
    pub(crate) fn create_scope(&mut self, scope: ScopeId, template: &'b VNode<'b>) -> usize {
        self.scope_stack.push(scope);
        let out = self.create(template);
        self.scope_stack.pop();
        out
    }

    /// Create this template and write its mutations
    pub(crate) fn create(&mut self, node: &'b VNode<'b>) -> usize {
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

        // Intialize the root nodes slice
        node.root_ids
            .intialize(vec![ElementId(0); node.template.get().roots.len()].into_boxed_slice());

        // The best renderers will have templates prehydrated and registered
        // Just in case, let's create the template using instructions anyways
        self.register_template(node.template.get());

        // we know that this will generate at least one mutation per node
        self.mutations
            .edits
            .reserve(node.template.get().roots.len());

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
                    self.write_dynamic_root(node, *id)
                }
                Element { .. } => {
                    #[cfg(not(debug_assertions))]
                    let id = self.write_element_root(node, idx, &mut attrs, &mut nodes, &[]);
                    #[cfg(debug_assertions)]
                    let id =
                        self.write_element_root(node, idx, &mut attrs, &mut nodes, &nodes_sorted);
                    id
                }
                Text { .. } => self.write_static_text_root(node, idx),
            })
            .sum()
    }

    fn write_static_text_root(&mut self, node: &VNode, idx: usize) -> usize {
        // Simply just load the template root, no modifications needed
        self.load_template_root(node, idx);

        // Text producs just one node on the stack
        1
    }

    fn write_dynamic_root(&mut self, template: &'b VNode<'b>, idx: usize) -> usize {
        use DynamicNode::*;
        match &template.dynamic_nodes[idx] {
            node @ Component { .. } | node @ Fragment(_) => {
                self.create_dynamic_node(template, node, idx)
            }
            Placeholder(VPlaceholder { id }) => {
                let id = self.set_slot(template, id, idx);
                self.mutations.push(CreatePlaceholder { id });
                1
            }
            Text(VText { id, value }) => {
                let id = self.set_slot(template, id, idx);
                self.create_static_text(value, id);
                1
            }
        }
    }

    fn create_static_text(&mut self, value: &str, id: ElementId) {
        // Safety: we promise not to re-alias this text later on after committing it to the mutation
        let unbounded_text: &str = unsafe { std::mem::transmute(value) };
        self.mutations.push(CreateTextNode {
            value: unbounded_text,
            id,
        });
    }

    /// We write all the descndent data for this element
    ///
    /// Elements can contain other nodes - and those nodes can be dynamic or static
    ///
    /// We want to make sure we write these nodes while on top of the root
    fn write_element_root(
        &mut self,
        template: &'b VNode<'b>,
        root_idx: usize,
        dynamic_attrs: &mut Peekable<impl Iterator<Item = (usize, &'static [u8])>>,
        dynamic_nodes_iter: &mut Peekable<impl Iterator<Item = ((usize, usize), &'static [u8])>>,
        dynamic_nodes: &[(usize, &'static [u8])],
    ) -> usize {
        // Load the template root and get the ID for the node on the stack
        let root_on_stack = self.load_template_root(template, root_idx);

        // Write all the attributes below this root
        self.write_attrs_on_root(dynamic_attrs, root_idx as u8, root_on_stack, template);

        // Load in all of the placeholder or dynamic content under this root too
        self.load_placeholders(dynamic_nodes_iter, dynamic_nodes, root_idx as u8, template);

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
        template: &'b VNode<'b>,
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
            let m = self.create_dynamic_node(template, &template.dynamic_nodes[idx], idx);
            if m > 0 {
                // The path is one shorter because the top node is the root
                let path = &template.template.get().node_paths[idx][1..];
                self.mutations.push(ReplacePlaceholder { m, path });
            }
        }
    }

    fn write_attrs_on_root(
        &mut self,
        attrs: &mut Peekable<impl Iterator<Item = (usize, &'static [u8])>>,
        root_idx: u8,
        root: ElementId,
        node: &VNode,
    ) {
        while let Some((mut attr_id, path)) =
            attrs.next_if(|(_, p)| p.first().copied() == Some(root_idx))
        {
            let id = self.assign_static_node_as_dynamic(path, root, node, attr_id);

            loop {
                self.write_attribute(&node.dynamic_attrs[attr_id], id);

                // Only push the dynamic attributes forward if they match the current path (same element)
                match attrs.next_if(|(_, p)| *p == path) {
                    Some((next_attr_id, _)) => attr_id = next_attr_id,
                    None => break,
                }
            }
        }
    }

    fn write_attribute(&mut self, attribute: &'b crate::Attribute<'b>, id: ElementId) {
        // Make sure we set the attribute's associated id
        attribute.mounted_element.set(id);

        // Safety: we promise not to re-alias this text later on after committing it to the mutation
        let unbounded_name: &str = unsafe { std::mem::transmute(attribute.name) };

        match &attribute.value {
            AttributeValue::Listener(_) => {
                self.mutations.push(NewEventListener {
                    // all listeners start with "on"
                    name: &unbounded_name[2..],
                    id,
                })
            }
            _ => {
                // Safety: we promise not to re-alias this text later on after committing it to the mutation
                let value: BorrowedAttributeValue<'b> = (&attribute.value).into();
                let unbounded_value = unsafe { std::mem::transmute(value) };

                self.mutations.push(SetAttribute {
                    name: unbounded_name,
                    value: unbounded_value,
                    ns: attribute.namespace,
                    id,
                })
            }
        }
    }

    fn load_template_root(&mut self, template: &VNode, root_idx: usize) -> ElementId {
        // Get an ID for this root since it's a real root
        let this_id = self.next_root(template, root_idx);
        template.root_ids.set(root_idx, this_id);

        self.mutations.push(LoadTemplate {
            name: template.template.get().name,
            index: root_idx,
            id: this_id,
        });

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
        template: &VNode,
        attr_id: usize,
    ) -> ElementId {
        if path.len() == 1 {
            return this_id;
        }

        // if attribute is on a root node, then we've already created the element
        // Else, it's deep in the template and we should create a new id for it
        let id = self.next_element(template, template.template.get().attr_paths[attr_id]);

        self.mutations.push(Mutation::AssignId {
            path: &path[1..],
            id,
        });

        id
    }

    /// Insert a new template into the VirtualDom's template registry
    pub(crate) fn register_template_first_byte_index(&mut self, mut template: Template<'static>) {
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
            self.mutations.templates.push(template);
        }
    }

    /// Insert a new template into the VirtualDom's template registry
    // used in conditional compilation
    #[allow(unused_mut)]
    pub(crate) fn register_template(&mut self, mut template: Template<'static>) {
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
                self.mutations.templates.push(template);
            }
        }
    }

    pub(crate) fn create_dynamic_node(
        &mut self,
        template: &'b VNode<'b>,
        node: &'b DynamicNode<'b>,
        idx: usize,
    ) -> usize {
        use DynamicNode::*;
        match node {
            Text(text) => self.create_dynamic_text(template, text, idx),
            Placeholder(place) => self.create_placeholder(place, template, idx),
            Component(component) => self.create_component_node(template, component, idx),
            Fragment(frag) => frag.iter().map(|child| self.create(child)).sum(),
        }
    }

    fn create_dynamic_text(
        &mut self,
        template: &'b VNode<'b>,
        text: &'b VText<'b>,
        idx: usize,
    ) -> usize {
        // Allocate a dynamic element reference for this text node
        let new_id = self.next_element(template, template.template.get().node_paths[idx]);

        // Make sure the text node is assigned to the correct element
        text.id.set(Some(new_id));

        // Safety: we promise not to re-alias this text later on after committing it to the mutation
        let value = unsafe { std::mem::transmute(text.value) };

        // Add the mutation to the list
        self.mutations.push(HydrateText {
            id: new_id,
            path: &template.template.get().node_paths[idx][1..],
            value,
        });

        // Since we're hydrating an existing node, we don't create any new nodes
        0
    }

    pub(crate) fn create_placeholder(
        &mut self,
        placeholder: &VPlaceholder,
        template: &'b VNode<'b>,
        idx: usize,
    ) -> usize {
        // Allocate a dynamic element reference for this text node
        let id = self.next_element(template, template.template.get().node_paths[idx]);

        // Make sure the text node is assigned to the correct element
        placeholder.id.set(Some(id));

        // Assign the ID to the existing node in the template
        self.mutations.push(AssignId {
            path: &template.template.get().node_paths[idx][1..],
            id,
        });

        // Since the placeholder is already in the DOM, we don't create any new nodes
        0
    }

    pub(super) fn create_component_node(
        &mut self,
        template: &'b VNode<'b>,
        component: &'b VComponent<'b>,
        idx: usize,
    ) -> usize {
        use RenderReturn::*;

        // Load up a ScopeId for this vcomponent
        let scope = self.load_scope_from_vcomponent(component);

        component.scope.set(Some(scope));

        match unsafe { self.run_scope(scope).extend_lifetime_ref() } {
            Ready(t) => self.mount_component(scope, template, t, idx),
            Aborted(t) => self.mount_aborted(template, t),
            Pending(_) => self.mount_async(template, idx, scope),
        }
    }

    /// Load a scope from a vcomponent. If the props don't exist, that means the component is currently "live"
    fn load_scope_from_vcomponent(&mut self, component: &VComponent) -> ScopeId {
        component
            .props
            .take()
            .map(|props| {
                let unbounded_props: Box<dyn AnyProps> = unsafe { std::mem::transmute(props) };
                self.new_scope(unbounded_props, component.name).id
            })
            .unwrap_or_else(|| component.scope.get().unwrap())
    }

    fn mount_component(
        &mut self,
        scope: ScopeId,
        parent: &'b VNode<'b>,
        new: &'b VNode<'b>,
        idx: usize,
    ) -> usize {
        // Keep track of how many mutations are in the buffer in case we need to split them out if a suspense boundary
        // is encountered
        let mutations_to_this_point = self.mutations.edits.len();

        // Create the component's root element
        let created = self.create_scope(scope, new);

        // If there are no suspense leaves below us, then just don't bother checking anything suspense related
        if self.collected_leaves.is_empty() {
            return created;
        }

        // If running the scope has collected some leaves and *this* component is a boundary, then handle the suspense
        let boundary = match self.scopes[scope].has_context::<Rc<SuspenseContext>>() {
            Some(boundary) => boundary,
            _ => return created,
        };

        // Since this is a boundary, use its placeholder within the template as the placeholder for the suspense tree
        let new_id = self.next_element(new, parent.template.get().node_paths[idx]);

        // Now connect everything to the boundary
        self.scopes[scope].placeholder.set(Some(new_id));

        // This involves breaking off the mutations to this point, and then creating a new placeholder for the boundary
        // Note that we break off dynamic mutations only - since static mutations aren't rendered immediately
        let split_off = unsafe {
            std::mem::transmute::<Vec<Mutation>, Vec<Mutation>>(
                self.mutations.edits.split_off(mutations_to_this_point),
            )
        };
        boundary.mutations.borrow_mut().edits.extend(split_off);
        boundary.created_on_stack.set(created);
        boundary
            .waiting_on
            .borrow_mut()
            .extend(self.collected_leaves.drain(..));

        // Now assign the placeholder in the DOM
        self.mutations.push(AssignId {
            id: new_id,
            path: &parent.template.get().node_paths[idx][1..],
        });

        0
    }

    fn mount_aborted(&mut self, parent: &'b VNode<'b>, placeholder: &VPlaceholder) -> usize {
        let id = self.next_element(parent, &[]);
        self.mutations.push(Mutation::CreatePlaceholder { id });
        placeholder.id.set(Some(id));
        1
    }

    /// Take the rendered nodes from a component and handle them if they were async
    ///
    /// IE simply assign an ID to the placeholder
    fn mount_async(&mut self, template: &VNode, idx: usize, scope: ScopeId) -> usize {
        let new_id = self.next_element(template, template.template.get().node_paths[idx]);

        // Set the placeholder of the scope
        self.scopes[scope].placeholder.set(Some(new_id));

        // Since the placeholder is already in the DOM, we don't create any new nodes
        self.mutations.push(AssignId {
            id: new_id,
            path: &template.template.get().node_paths[idx][1..],
        });

        0
    }

    fn set_slot(
        &mut self,
        template: &'b VNode<'b>,
        slot: &'b Cell<Option<ElementId>>,
        id: usize,
    ) -> ElementId {
        let id = self.next_element(template, template.template.get().node_paths[id]);
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
