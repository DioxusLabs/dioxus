use crate::factory::RenderReturn;
use crate::innerlude::{Mutations, SuspenseContext};
use crate::mutations::Mutation;
use crate::mutations::Mutation::*;
use crate::nodes::VNode;
use crate::nodes::{DynamicNode, TemplateNode};
use crate::virtual_dom::VirtualDom;
use crate::{AttributeValue, ElementId, ScopeId, TemplateAttribute};

impl VirtualDom {
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
            for node in template.template.roots {
                let mutations = &mut mutations.template_mutations;
                self.create_static_node(mutations, template, node);
            }

            mutations.template_mutations.push(SaveTemplate {
                name: template.template.id,
                m: template.template.roots.len(),
            });

            self.templates
                .insert(template.template.id, template.template.clone());
        }

        // Walk the roots, creating nodes and assigning IDs
        // todo: adjust dynamic nodes to be in the order of roots and then leaves (ie BFS)
        let mut dynamic_attrs = template.template.attr_paths.iter().enumerate().peekable();
        let mut dynamic_nodes = template.template.node_paths.iter().enumerate().peekable();

        let cur_scope = self.scope_stack.last().copied().unwrap();

        println!("creating template: {:#?}", template);

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
                    let dynamic_node = &template.dynamic_nodes[*id];

                    match dynamic_node {
                        DynamicNode::Fragment { .. } | DynamicNode::Component { .. } => self
                            .create_dynamic_node(
                                mutations,
                                template,
                                &template.dynamic_nodes[*id],
                                *id,
                            ),
                        DynamicNode::Text {
                            id: slot, value, ..
                        } => {
                            let id = self.next_element(template);
                            slot.set(id);
                            mutations.push(CreateTextNode {
                                value: value.clone(),
                                id,
                            });
                            1
                        }
                        DynamicNode::Placeholder(id) => {
                            let id = self.next_element(template);
                            mutations.push(CreatePlaceholder { id });
                            1
                        }
                    }
                    // self.create_dynamic_node(mutations, template, &template.dynamic_nodes[*id], *id)
                }
            };

            // we're on top of a node that has a dynamic attribute for a descendant
            // Set that attribute now before the stack gets in a weird state
            while let Some((mut attr_id, path)) =
                dynamic_attrs.next_if(|(_, p)| p[0] == root_idx as u8)
            {
                let id = self.next_element(template);
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
                            event_name: attribute.name,
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

            // todo:
            //
            //  we walk the roots front to back when creating nodes, bur want to fill in the dynamic nodes
            // back to front. This is because the indices would shift around because the paths become invalid
            //
            // We could easily implement this without the vec by walking the indicies forward
            let mut queued_changes = vec![];

            // We're on top of a node that has a dynamic child for a descendant
            // Skip any node that's a root
            while let Some((idx, path)) =
                dynamic_nodes.next_if(|(_, p)| p.len() > 1 && p[0] == root_idx as u8)
            {
                let node = &template.dynamic_nodes[idx];
                let m = self.create_dynamic_node(mutations, template, node, idx);
                if m > 0 {
                    queued_changes.push(ReplacePlaceholder {
                        m,
                        path: &path[1..],
                    });
                }
            }

            for change in queued_changes.into_iter().rev() {
                mutations.push(change);
            }
        }

        on_stack
    }

    pub(crate) fn create_static_node<'a>(
        &mut self,
        mutations: &mut Vec<Mutation<'a>>,
        template: &'a VNode<'a>,
        node: &'a TemplateNode<'static>,
    ) {
        match *node {
            // Todo: create the children's template
            TemplateNode::Dynamic(_) => {
                let id = self.next_element(template);
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
                let id = self.next_element(template);

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
        match &node {
            DynamicNode::Text { id, value, inner } => {
                let new_id = self.next_element(template);
                id.set(new_id);
                mutations.push(HydrateText {
                    id: new_id,
                    path: &template.template.node_paths[idx][1..],
                    value,
                });
                0
            }

            DynamicNode::Component {
                props,
                placeholder,
                scope: scope_slot,
                ..
            } => {
                let scope = self
                    .new_scope(unsafe { std::mem::transmute(props.get()) })
                    .id;

                scope_slot.set(Some(scope));

                let return_nodes = unsafe { self.run_scope(scope).extend_lifetime_ref() };

                match return_nodes {
                    RenderReturn::Sync(None) | RenderReturn::Async(_) => {
                        let new_id = self.next_element(template);
                        placeholder.set(Some(new_id));
                        self.scopes[scope.0].placeholder.set(Some(new_id));
                        mutations.push(AssignId {
                            id: new_id,
                            path: &template.template.node_paths[idx][1..],
                        });
                        0
                    }

                    RenderReturn::Sync(Some(template)) => {
                        let mutations_to_this_point = mutations.len();

                        self.scope_stack.push(scope);
                        let mut created = self.create(mutations, template);
                        self.scope_stack.pop();

                        if !self.collected_leaves.is_empty() {
                            if let Some(boundary) =
                                self.scopes[scope.0].has_context::<SuspenseContext>()
                            {
                                let mut boundary_mut = boundary.borrow_mut();
                                let split_off = mutations.split_off(mutations_to_this_point);

                                let split_off = unsafe { std::mem::transmute(split_off) };

                                boundary_mut.mutations.edits = split_off;
                                boundary_mut
                                    .waiting_on
                                    .extend(self.collected_leaves.drain(..));

                                // Since this is a boundary, use it as a placeholder
                                let new_id = self.next_element(template);
                                placeholder.set(Some(new_id));
                                self.scopes[scope.0].placeholder.set(Some(new_id));
                                mutations.push(AssignId {
                                    id: new_id,
                                    path: &template.template.node_paths[idx][1..],
                                });
                                created = 0;
                            }
                        }

                        // handle any waiting on futures accumulated by async calls down the tree
                        // if this is a boundary, we split off the tree
                        created
                    }
                }
            }

            DynamicNode::Fragment { nodes, .. } => {
                //
                nodes
                    .iter()
                    .fold(0, |acc, child| acc + self.create(mutations, child))
            }

            DynamicNode::Placeholder(slot) => {
                let id = self.next_element(template);
                slot.set(id);
                mutations.push(AssignId {
                    path: &template.template.node_paths[idx][1..],
                    id,
                });

                0
            }
        }
    }
}
