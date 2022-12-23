use crate::any_props::AnyProps;
use crate::innerlude::{VComponent, VPlaceholder, VText};
use crate::mutations::Mutation;
use crate::mutations::Mutation::*;
use crate::nodes::VNode;
use crate::nodes::{DynamicNode, TemplateNode};
use crate::virtual_dom::VirtualDom;
use crate::{AttributeValue, ElementId, RenderReturn, ScopeId, SuspenseContext};
use std::cell::Cell;
use std::iter::{Enumerate, Peekable};
use std::rc::Rc;
use std::slice;
use TemplateNode::*;

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
        // The best renderers will have templates prehydrated and registered
        // Just in case, let's create the template using instructions anyways
        if !self.templates.contains_key(&node.template.name) {
            self.register_template(node);
        }

        // we know that this will generate at least one mutation per node
        self.mutations.edits.reserve(node.template.roots.len());

        // Walk the roots, creating nodes and assigning IDs
        // todo: adjust dynamic nodes to be in the order of roots and then leaves (ie BFS)
        let mut attrs = node.template.attr_paths.iter().enumerate().peekable();
        let mut nodes = node.template.node_paths.iter().enumerate().peekable();

        node.template
            .roots
            .iter()
            .enumerate()
            .map(|(idx, root)| match root {
                DynamicText { id } | Dynamic { id } => {
                    nodes.next().unwrap();
                    self.write_dynamic_root(node, *id)
                }
                Element { .. } => self.write_element_root(node, idx, &mut attrs, &mut nodes),
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
            node @ Fragment(_) => self.create_dynamic_node(template, node, idx),
            node @ Component { .. } => self.create_dynamic_node(template, node, idx),
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
        dynamic_attrs: &mut Peekable<Enumerate<slice::Iter<&'static [u8]>>>,
        dynamic_nodes: &mut Peekable<Enumerate<slice::Iter<&'static [u8]>>>,
    ) -> usize {
        // Load the template root and get the ID for the node on the stack
        let root_on_stack = self.load_template_root(template, root_idx);

        // Write all the attributes below this root
        self.write_attrs_on_root(dynamic_attrs, root_idx, root_on_stack, template);

        // Load in all of the placeholder or dynamic content under this root too
        self.load_placeholders(dynamic_nodes, root_idx, template);

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
    fn load_placeholders(
        &mut self,
        dynamic_nodes: &mut Peekable<Enumerate<slice::Iter<&'static [u8]>>>,
        root_idx: usize,
        template: &'b VNode<'b>,
    ) {
        let (start, end) = match collect_dyn_node_range(dynamic_nodes, root_idx) {
            Some((a, b)) => (a, b),
            None => return,
        };

        for idx in (start..=end).rev() {
            let m = self.create_dynamic_node(template, &template.dynamic_nodes[idx], idx);
            if m > 0 {
                // The path is one shorter because the top node is the root
                let path = &template.template.node_paths[idx][1..];
                self.mutations.push(ReplacePlaceholder { m, path });
            }
        }
    }

    fn write_attrs_on_root(
        &mut self,
        attrs: &mut Peekable<Enumerate<slice::Iter<&'static [u8]>>>,
        root_idx: usize,
        root: ElementId,
        node: &VNode,
    ) {
        while let Some((mut attr_id, path)) = attrs.next_if(|(_, p)| p[0] == root_idx as u8) {
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

    fn write_attribute(&mut self, attribute: &crate::Attribute, id: ElementId) {
        // Make sure we set the attribute's associated id
        attribute.mounted_element.set(id);

        // Safety: we promise not to re-alias this text later on after committing it to the mutation
        let unbounded_name: &str = unsafe { std::mem::transmute(attribute.name) };

        match &attribute.value {
            AttributeValue::Text(value) => {
                // Safety: we promise not to re-alias this text later on after committing it to the mutation
                let unbounded_value: &str = unsafe { std::mem::transmute(*value) };

                self.mutations.push(SetAttribute {
                    name: unbounded_name,
                    value: unbounded_value,
                    ns: attribute.namespace,
                    id,
                })
            }
            AttributeValue::Bool(value) => self.mutations.push(SetBoolAttribute {
                name: unbounded_name,
                value: *value,
                id,
            }),
            AttributeValue::Listener(_) => {
                self.mutations.push(NewEventListener {
                    // all listeners start with "on"
                    name: &unbounded_name[2..],
                    id,
                })
            }
            AttributeValue::Float(_) => todo!(),
            AttributeValue::Int(_) => todo!(),
            AttributeValue::Any(_) => todo!(),
            AttributeValue::None => todo!(),
        }
    }

    fn load_template_root(&mut self, template: &VNode, root_idx: usize) -> ElementId {
        // Get an ID for this root since it's a real root
        let this_id = self.next_root(template, root_idx);
        template.root_ids[root_idx].set(Some(this_id));

        self.mutations.push(LoadTemplate {
            name: template.template.name,
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
        let id = self.next_element(template, template.template.attr_paths[attr_id]);

        self.mutations.push(Mutation::AssignId {
            path: &path[1..],
            id,
        });

        id
    }

    /// Insert a new template into the VirtualDom's template registry
    fn register_template(&mut self, template: &'b VNode<'b>) {
        // First, make sure we mark the template as seen, regardless if we process it
        self.templates
            .insert(template.template.name, template.template);

        // If it's all dynamic nodes, then we don't need to register it
        if !template.template.is_completely_dynamic() {
            self.mutations.templates.push(template.template);
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
            Fragment(frag) => self.create_fragment(frag),
            Placeholder(frag) => self.create_placeholder(frag, template, idx),
            Component(component) => self.create_component_node(template, component, idx),
        }
    }

    fn create_dynamic_text(
        &mut self,
        template: &'b VNode<'b>,
        text: &'b VText<'b>,
        idx: usize,
    ) -> usize {
        // Allocate a dynamic element reference for this text node
        let new_id = self.next_element(template, template.template.node_paths[idx]);

        // Make sure the text node is assigned to the correct element
        text.id.set(Some(new_id));

        // Safety: we promise not to re-alias this text later on after committing it to the mutation
        let value = unsafe { std::mem::transmute(text.value) };

        // Add the mutation to the list
        self.mutations.push(HydrateText {
            id: new_id,
            path: &template.template.node_paths[idx][1..],
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
        let id = self.next_element(template, template.template.node_paths[idx]);

        // Make sure the text node is assigned to the correct element
        placeholder.id.set(Some(id));

        // Assign the ID to the existing node in the template
        self.mutations.push(AssignId {
            path: &template.template.node_paths[idx][1..],
            id,
        });

        // Since the placeholder is already in the DOM, we don't create any new nodes
        0
    }

    pub(crate) fn create_fragment(&mut self, nodes: &'b [VNode<'b>]) -> usize {
        nodes.iter().map(|child| self.create(child)).sum()
    }

    pub(super) fn create_component_node(
        &mut self,
        template: &'b VNode<'b>,
        component: &'b VComponent<'b>,
        idx: usize,
    ) -> usize {
        let scope = match component.props.take() {
            Some(props) => {
                let unbounded_props: Box<dyn AnyProps> = unsafe { std::mem::transmute(props) };
                let scope = self.new_scope(unbounded_props, component.name);
                scope.id
            }

            // Component is coming back, it probably still exists, right?
            None => component.scope.get().unwrap(),
        };

        component.scope.set(Some(scope));

        let return_nodes = unsafe { self.run_scope(scope).extend_lifetime_ref() };

        use RenderReturn::*;

        match return_nodes {
            Sync(Some(t)) => self.mount_component(scope, template, t, idx),
            Sync(None) => todo!("Propogate error upwards"),
            Async(_) => self.mount_component_placeholder(template, idx, scope),
        }
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
        let boundary = match self.scopes[scope.0].has_context::<Rc<SuspenseContext>>() {
            Some(boundary) => boundary,
            _ => return created,
        };

        // Since this is a boundary, use its placeholder within the template as the placeholder for the suspense tree
        let new_id = self.next_element(new, parent.template.node_paths[idx]);

        // Now connect everything to the boundary
        self.scopes[scope.0].placeholder.set(Some(new_id));

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
            path: &parent.template.node_paths[idx][1..],
        });

        0
    }

    /// Take the rendered nodes from a component and handle them if they were async
    ///
    /// IE simply assign an ID to the placeholder
    fn mount_component_placeholder(
        &mut self,
        template: &VNode,
        idx: usize,
        scope: ScopeId,
    ) -> usize {
        let new_id = self.next_element(template, template.template.node_paths[idx]);

        // Set the placeholder of the scope
        self.scopes[scope.0].placeholder.set(Some(new_id));

        // Since the placeholder is already in the DOM, we don't create any new nodes
        self.mutations.push(AssignId {
            id: new_id,
            path: &template.template.node_paths[idx][1..],
        });

        0
    }

    fn set_slot(
        &mut self,
        template: &'b VNode<'b>,
        slot: &'b Cell<Option<ElementId>>,
        id: usize,
    ) -> ElementId {
        let id = self.next_element(template, template.template.node_paths[id]);
        slot.set(Some(id));
        id
    }
}

fn collect_dyn_node_range(
    dynamic_nodes: &mut Peekable<Enumerate<slice::Iter<&[u8]>>>,
    root_idx: usize,
) -> Option<(usize, usize)> {
    let start = match dynamic_nodes.peek() {
        Some((idx, p)) if p[0] == root_idx as u8 => *idx,
        _ => return None,
    };

    let mut end = start;

    while let Some((idx, p)) = dynamic_nodes.next_if(|(_, p)| p[0] == root_idx as u8) {
        if p.len() == 1 {
            continue;
        }

        end = idx;
    }

    Some((start, end))
}
