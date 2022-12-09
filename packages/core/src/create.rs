use std::cell::Cell;
use std::rc::Rc;

use crate::innerlude::{VComponent, VText};
use crate::mutations::Mutation;
use crate::mutations::Mutation::*;
use crate::nodes::VNode;
use crate::nodes::{DynamicNode, TemplateNode};
use crate::virtual_dom::VirtualDom;
use crate::{AttributeValue, ElementId, RenderReturn, ScopeId, SuspenseContext};

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
    pub(crate) fn create(&mut self, template: &'b VNode<'b>) -> usize {
        // The best renderers will have templates prehydrated and registered
        // Just in case, let's create the template using instructions anyways
        if !self.templates.contains_key(&template.template.name) {
            self.register_template(template);
        }

        // Walk the roots, creating nodes and assigning IDs
        // todo: adjust dynamic nodes to be in the order of roots and then leaves (ie BFS)
        let mut dynamic_attrs = template.template.attr_paths.iter().enumerate().peekable();
        let mut dynamic_nodes = template.template.node_paths.iter().enumerate().peekable();

        let cur_scope = self.scope_stack.last().copied().unwrap();

        // we know that this will generate at least one mutation per node
        self.mutations.edits.reserve(template.template.roots.len());

        let mut on_stack = 0;
        for (root_idx, root) in template.template.roots.iter().enumerate() {
            // We might need to generate an ID for the root node
            on_stack += match root {
                TemplateNode::DynamicText { id } | TemplateNode::Dynamic { id } => {
                    match &template.dynamic_nodes[*id] {
                        // a dynamic text node doesn't replace a template node, instead we create it on the fly
                        DynamicNode::Text(VText { id: slot, value }) => {
                            let id = self.next_element(template, template.template.node_paths[*id]);
                            slot.set(id);

                            // Safety: we promise not to re-alias this text later on after committing it to the mutation
                            let unbounded_text = unsafe { std::mem::transmute(*value) };
                            self.mutations.push(CreateTextNode {
                                value: unbounded_text,
                                id,
                            });

                            1
                        }

                        DynamicNode::Placeholder(slot) => {
                            let id = self.next_element(template, template.template.node_paths[*id]);
                            slot.set(id);
                            self.mutations.push(CreatePlaceholder { id });
                            1
                        }

                        DynamicNode::Fragment(_) | DynamicNode::Component { .. } => {
                            self.create_dynamic_node(template, &template.dynamic_nodes[*id], *id)
                        }
                    }
                }

                TemplateNode::Element { .. } | TemplateNode::Text { .. } => {
                    let this_id = self.next_root(template, root_idx);

                    template.root_ids[root_idx].set(this_id);
                    self.mutations.push(LoadTemplate {
                        name: template.template.name,
                        index: root_idx,
                        id: this_id,
                    });

                    // we're on top of a node that has a dynamic attribute for a descendant
                    // Set that attribute now before the stack gets in a weird state
                    while let Some((mut attr_id, path)) =
                        dynamic_attrs.next_if(|(_, p)| p[0] == root_idx as u8)
                    {
                        // if attribute is on a root node, then we've already created the element
                        // Else, it's deep in the template and we should create a new id for it
                        let id = match path.len() {
                            1 => this_id,
                            _ => {
                                let id = self
                                    .next_element(template, template.template.attr_paths[attr_id]);
                                self.mutations.push(Mutation::AssignId {
                                    path: &path[1..],
                                    id,
                                });
                                id
                            }
                        };

                        loop {
                            let attribute = template.dynamic_attrs.get(attr_id).unwrap();
                            attribute.mounted_element.set(id);

                            // Safety: we promise not to re-alias this text later on after committing it to the mutation
                            let unbounded_name: &str =
                                unsafe { std::mem::transmute(attribute.name) };

                            match &attribute.value {
                                AttributeValue::Listener(_) => {
                                    self.mutations.push(NewEventListener {
                                        // all listeners start with "on"
                                        name: &unbounded_name[2..],
                                        scope: cur_scope,
                                        id,
                                    })
                                }
                                _ => {
                                    // Safety: we promise not to re-alias this text later on after committing it to the mutation
                                    let unbounded_value =
                                        unsafe { std::mem::transmute(attribute.value.clone()) };

                                    self.mutations.push(SetAttribute {
                                        name: unbounded_name,
                                        value: unbounded_value,
                                        ns: attribute.namespace,
                                        id,
                                    })
                                }
                            }

                            // Only push the dynamic attributes forward if they match the current path (same element)
                            match dynamic_attrs.next_if(|(_, p)| *p == path) {
                                Some((next_attr_id, _)) => attr_id = next_attr_id,
                                None => break,
                            }
                        }
                    }

                    // We're on top of a node that has a dynamic child for a descendant
                    // Skip any node that's a root
                    let mut start = None;
                    let mut end = None;

                    // Collect all the dynamic nodes below this root
                    // We assign the start and end of the range of dynamic nodes since they area ordered in terms of tree path
                    //
                    // [0]
                    // [1, 1]     <---|
                    // [1, 1, 1]  <---| these are the range of dynamic nodes below root 1
                    // [1, 1, 2]  <---|
                    // [2]
                    //
                    // We collect each range and then create them and replace the placeholder in the template
                    while let Some((idx, p)) =
                        dynamic_nodes.next_if(|(_, p)| p[0] == root_idx as u8)
                    {
                        if p.len() == 1 {
                            continue;
                        }

                        if start.is_none() {
                            start = Some(idx);
                        }

                        end = Some(idx);
                    }

                    //
                    if let (Some(start), Some(end)) = (start, end) {
                        for idx in start..=end {
                            let node = &template.dynamic_nodes[idx];
                            let m = self.create_dynamic_node(template, node, idx);
                            if m > 0 {
                                self.mutations.push(ReplacePlaceholder {
                                    m,
                                    path: &template.template.node_paths[idx][1..],
                                });
                            }
                        }
                    }

                    // elements create only one node :-)
                    1
                }
            };
        }

        on_stack
    }

    /// Insert a new template into the VirtualDom's template registry
    fn register_template(&mut self, template: &'b VNode<'b>) {
        // First, make sure we mark the template as seen, regardless if we process it
        self.templates
            .insert(template.template.name, template.template);

        // If it's all dynamic nodes, then we don't need to register it
        // Quickly run through and see if it's all just dynamic nodes
        if template.template.roots.iter().all(|root| {
            matches!(
                root,
                TemplateNode::Dynamic { .. } | TemplateNode::DynamicText { .. }
            )
        }) {
            return;
        }

        self.mutations.templates.push(template.template);
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
        text.id.set(new_id);

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
        slot: &Cell<ElementId>,
        template: &'b VNode<'b>,
        idx: usize,
    ) -> usize {
        // Allocate a dynamic element reference for this text node
        let id = self.next_element(template, template.template.node_paths[idx]);

        // Make sure the text node is assigned to the correct element
        slot.set(id);

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
        let props = component
            .props
            .take()
            .expect("Props to always exist when a component is being created");

        let unbounded_props = unsafe { std::mem::transmute(props) };

        let scope = self.new_scope(unbounded_props, component.name);
        let scope = scope.id;
        component.scope.set(Some(scope));

        let return_nodes = unsafe { self.run_scope(scope).extend_lifetime_ref() };

        use RenderReturn::*;

        match return_nodes {
            Sync(Ok(t)) => self.mount_component(scope, template, t, idx),
            Sync(Err(_e)) => todo!("Propogate error upwards"),
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
}
