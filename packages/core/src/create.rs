use std::cell::Cell;

use crate::factory::RenderReturn;
use crate::innerlude::{Mutations, VComponent, VFragment, VText};
use crate::mutations::Mutation;
use crate::mutations::Mutation::*;
use crate::nodes::VNode;
use crate::nodes::{DynamicNode, TemplateNode};
use crate::virtual_dom::VirtualDom;
use crate::{AttributeValue, ElementId, ScopeId, SuspenseContext, TemplateAttribute};

impl VirtualDom {
    /// Create a new template [`VNode`] and write it to the [`Mutations`] buffer.
    ///
    /// This method pushes the ScopeID to the internal scopestack and returns the number of nodes created.
    pub(crate) fn create_scope<'a>(
        &mut self,
        scope: ScopeId,
        mutations: &mut Mutations<'a>,
        template: &'a VNode<'a>,
    ) -> usize {
        self.scope_stack.push(scope);
        let out = self.create(mutations, template);
        self.scope_stack.pop();

        out
    }

    /// Create this template and write its mutations
    pub(crate) fn create<'a>(
        &mut self,
        mutations: &mut Mutations<'a>,
        template: &'a VNode<'a>,
    ) -> usize {
        // The best renderers will have templates prehydrated and registered
        // Just in case, let's create the template using instructions anyways
        if !self.templates.contains_key(&template.template.id) {
            self.register_template(template, mutations);
        }

        // Walk the roots, creating nodes and assigning IDs
        // todo: adjust dynamic nodes to be in the order of roots and then leaves (ie BFS)
        let mut dynamic_attrs = template.template.attr_paths.iter().enumerate().peekable();
        let mut dynamic_nodes = template.template.node_paths.iter().enumerate().peekable();

        let cur_scope = self.scope_stack.last().copied().unwrap();

        let mut on_stack = 0;
        for (root_idx, root) in template.template.roots.iter().enumerate() {
            on_stack += match root {
                TemplateNode::Element { .. } | TemplateNode::Text(_) => {
                    mutations.push(LoadTemplate {
                        name: template.template.id,
                        index: root_idx,
                    });
                    1
                }

                TemplateNode::DynamicText(id) | TemplateNode::Dynamic(id) => {
                    match &template.dynamic_nodes[*id] {
                        DynamicNode::Fragment { .. } | DynamicNode::Component { .. } => self
                            .create_dynamic_node(
                                mutations,
                                template,
                                &template.dynamic_nodes[*id],
                                *id,
                            ),
                        DynamicNode::Text(VText { id: slot, value }) => {
                            let id = self.next_element(template, template.template.node_paths[*id]);
                            slot.set(id);
                            mutations.push(CreateTextNode { value, id });
                            1
                        }
                        DynamicNode::Placeholder(slot) => {
                            let id = self.next_element(template, template.template.node_paths[*id]);
                            slot.set(id);
                            mutations.push(CreatePlaceholder { id });
                            1
                        }
                    }
                }
            };

            // we're on top of a node that has a dynamic attribute for a descendant
            // Set that attribute now before the stack gets in a weird state
            while let Some((mut attr_id, path)) =
                dynamic_attrs.next_if(|(_, p)| p[0] == root_idx as u8)
            {
                let id = self.next_element(template, template.template.attr_paths[attr_id]);
                mutations.push(AssignId {
                    path: &path[1..],
                    id,
                });

                loop {
                    let attribute = template.dynamic_attrs.get(attr_id).unwrap();
                    attribute.mounted_element.set(id);

                    match &attribute.value {
                        AttributeValue::Text(value) => mutations.push(SetAttribute {
                            name: attribute.name,
                            value: *value,
                            ns: attribute.namespace,
                            id,
                        }),
                        AttributeValue::Bool(value) => mutations.push(SetBoolAttribute {
                            name: attribute.name,
                            value: *value,
                            id,
                        }),
                        AttributeValue::Listener(_) => mutations.push(NewEventListener {
                            // all listeners start with "on"
                            event_name: &attribute.name[2..],
                            scope: cur_scope,
                            id,
                        }),
                        AttributeValue::Float(_) => todo!(),
                        AttributeValue::Int(_) => todo!(),
                        AttributeValue::Any(_) => todo!(),
                        AttributeValue::None => todo!(),
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

            while let Some((idx, p)) = dynamic_nodes.next_if(|(_, p)| p[0] == root_idx as u8) {
                if p.len() == 1 {
                    continue;
                }

                if start.is_none() {
                    start = Some(idx);
                }

                end = Some(idx);
            }

            if let (Some(start), Some(end)) = (start, end) {
                for idx in start..=end {
                    let node = &template.dynamic_nodes[idx];
                    let m = self.create_dynamic_node(mutations, template, node, idx);
                    if m > 0 {
                        mutations.push(ReplacePlaceholder {
                            m,
                            path: &template.template.node_paths[idx][1..],
                        });
                    }
                }
            }
        }

        on_stack
    }

    /// Insert a new template into the VirtualDom's template registry
    fn register_template<'a>(&mut self, template: &'a VNode<'a>, mutations: &mut Mutations<'a>) {
        for node in template.template.roots {
            self.create_static_node(&mut mutations.template_mutations, template, node);
        }

        mutations.template_mutations.push(SaveTemplate {
            name: template.template.id,
            m: template.template.roots.len(),
        });

        self.templates
            .insert(template.template.id, template.template);
    }

    pub(crate) fn create_static_node<'a>(
        &mut self,
        mutations: &mut Vec<Mutation<'a>>,
        template: &'a VNode<'a>,
        node: &'a TemplateNode<'static>,
    ) {
        match *node {
            // Todo: create the children's template
            TemplateNode::Dynamic(idx) => {
                let id = self.next_element(template, template.template.node_paths[idx]);
                mutations.push(CreatePlaceholder { id })
            }
            TemplateNode::Text(value) => mutations.push(CreateStaticText { value }),
            TemplateNode::DynamicText { .. } => mutations.push(CreateStaticText {
                value: "placeholder",
            }),

            TemplateNode::Element {
                attrs,
                children,
                namespace,
                tag,
                inner_opt,
            } => {
                let id = self.next_element(template, &[]); // never gets referenced, empty path is fine, I think?

                mutations.push(CreateElement {
                    name: tag,
                    namespace,
                    id,
                });

                mutations.extend(attrs.into_iter().filter_map(|attr| match attr {
                    TemplateAttribute::Static {
                        name,
                        value,
                        namespace,
                        ..
                    } => Some(SetAttribute {
                        name,
                        value,
                        id,
                        ns: *namespace,
                    }),
                    _ => None,
                }));

                if children.is_empty() && inner_opt {
                    return;
                }

                children
                    .into_iter()
                    .for_each(|child| self.create_static_node(mutations, template, child));

                mutations.push(AppendChildren { m: children.len() })
            }
        }
    }

    pub(crate) fn create_dynamic_node<'a>(
        &mut self,
        mutations: &mut Mutations<'a>,
        template: &'a VNode<'a>,
        node: &'a DynamicNode<'a>,
        idx: usize,
    ) -> usize {
        use DynamicNode::*;
        match node {
            Text(text) => self.create_dynamic_text(mutations, template, text, idx),
            Placeholder(slot) => self.create_placeholder(template, idx, slot, mutations),
            Fragment(frag) => self.create_fragment(frag, mutations),
            Component(component) => self.create_component_node(mutations, template, component, idx),
        }
    }

    fn create_dynamic_text<'a>(
        &mut self,
        mutations: &mut Mutations<'a>,
        template: &VNode<'a>,
        text: &VText<'a>,
        idx: usize,
    ) -> usize {
        // Allocate a dynamic element reference for this text node
        let new_id = self.next_element(template, template.template.node_paths[idx]);

        // Make sure the text node is assigned to the correct element
        text.id.set(new_id);

        // Add the mutation to the list
        mutations.push(HydrateText {
            id: new_id,
            path: &template.template.node_paths[idx][1..],
            value: text.value,
        });

        // Since we're hydrating an existing node, we don't create any new nodes
        0
    }

    fn create_placeholder(
        &mut self,
        template: &VNode,
        idx: usize,
        slot: &Cell<ElementId>,
        mutations: &mut Mutations,
    ) -> usize {
        // Allocate a dynamic element reference for this text node
        let id = self.next_element(template, template.template.node_paths[idx]);

        // Make sure the text node is assigned to the correct element
        slot.set(id);

        // Assign the ID to the existing node in the template
        mutations.push(AssignId {
            path: &template.template.node_paths[idx][1..],
            id,
        });

        // Since the placeholder is already in the DOM, we don't create any new nodes
        0
    }

    fn create_fragment<'a>(
        &mut self,
        frag: &'a VFragment<'a>,
        mutations: &mut Mutations<'a>,
    ) -> usize {
        frag.nodes
            .iter()
            .fold(0, |acc, child| acc + self.create(mutations, child))
    }

    fn create_component_node<'a>(
        &mut self,
        mutations: &mut Mutations<'a>,
        template: &'a VNode<'a>,
        component: &'a VComponent<'a>,
        idx: usize,
    ) -> usize {
        let props = component.props.replace(None).unwrap();

        let prop_ptr = unsafe { std::mem::transmute(props.as_ref()) };
        let scope = self.new_scope(prop_ptr).id;

        component.props.replace(Some(props));
        component.scope.set(Some(scope));

        let return_nodes = unsafe { self.run_scope(scope).extend_lifetime_ref() };

        use RenderReturn::*;

        match return_nodes {
            Sync(Some(t)) => self.mount_component(mutations, scope, t, idx),
            Sync(None) | Async(_) => {
                self.mount_component_placeholder(template, idx, scope, mutations)
            }
        }
    }

    fn mount_component<'a>(
        &mut self,
        mutations: &mut Mutations<'a>,
        scope: ScopeId,
        template: &'a VNode<'a>,
        idx: usize,
    ) -> usize {
        // Keep track of how many mutations are in the buffer in case we need to split them out if a suspense boundary
        // is encountered
        let mutations_to_this_point = mutations.len();

        // Create the component's root element
        let created = self.create_scope(scope, mutations, template);

        // If there are no suspense leaves below us, then just don't bother checking anything suspense related
        if self.collected_leaves.is_empty() {
            return created;
        }

        // If running the scope has collected some leaves and *this* component is a boundary, then handle the suspense
        let boundary = match self.scopes[scope.0].has_context::<SuspenseContext>() {
            Some(boundary) => boundary,
            _ => return created,
        };

        // Since this is a boundary, use its placeholder within the template as the placeholder for the suspense tree
        let new_id = self.next_element(template, template.template.node_paths[idx]);

        // Now connect everything to the boundary
        self.scopes[scope.0].placeholder.set(Some(new_id));

        // This involves breaking off the mutations to this point, and then creating a new placeholder for the boundary
        // Note that we break off dynamic mutations only - since static mutations aren't rendered immediately
        let split_off = unsafe {
            std::mem::transmute::<Vec<Mutation>, Vec<Mutation>>(
                mutations.split_off(mutations_to_this_point),
            )
        };
        boundary.mutations.borrow_mut().edits.extend(split_off);
        boundary.created_on_stack.set(created);
        boundary
            .waiting_on
            .borrow_mut()
            .extend(self.collected_leaves.drain(..));

        // Now assign the placeholder in the DOM
        mutations.push(AssignId {
            id: new_id,
            path: &template.template.node_paths[idx][1..],
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
        mutations: &mut Mutations,
    ) -> usize {
        let new_id = self.next_element(template, template.template.node_paths[idx]);

        // Set the placeholder of the scope
        self.scopes[scope.0].placeholder.set(Some(new_id));

        // Since the placeholder is already in the DOM, we don't create any new nodes
        mutations.push(AssignId {
            id: new_id,
            path: &template.template.node_paths[idx][1..],
        });

        0
    }
}
