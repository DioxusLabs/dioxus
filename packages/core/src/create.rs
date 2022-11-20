use crate::factory::RenderReturn;
use crate::innerlude::{Mutations, SuspenseContext, VText};
use crate::mutations::Mutation;
use crate::mutations::Mutation::*;
use crate::nodes::VNode;
use crate::nodes::{DynamicNode, TemplateNode};
use crate::virtual_dom::VirtualDom;
use crate::{AttributeValue, ScopeId, TemplateAttribute};

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
        match &node {
            DynamicNode::Text(VText { id, value }) => {
                let new_id = self.next_element(template, template.template.node_paths[idx]);
                id.set(new_id);
                mutations.push(HydrateText {
                    id: new_id,
                    path: &template.template.node_paths[idx][1..],
                    value,
                });
                0
            }

            DynamicNode::Component(component) => {
                let props = component.props.replace(None).unwrap();
                let prop_ptr = unsafe { std::mem::transmute(props.as_ref()) };
                let scope = self.new_scope(prop_ptr).id;
                component.props.replace(Some(props));

                component.scope.set(Some(scope));

                let return_nodes = unsafe { self.run_scope(scope).extend_lifetime_ref() };

                match return_nodes {
                    RenderReturn::Sync(None) => {
                        todo!()
                    }

                    RenderReturn::Async(_) => {
                        let new_id = self.next_element(template, template.template.node_paths[idx]);
                        component.placeholder.set(Some(new_id));
                        self.scopes[scope.0].placeholder.set(Some(new_id));

                        mutations.push(AssignId {
                            id: new_id,
                            path: &template.template.node_paths[idx][1..],
                        });

                        let boudary = self.scopes[scope.0]
                            .consume_context::<SuspenseContext>()
                            .unwrap();

                        if boudary.placeholder.get().is_none() {
                            boudary.placeholder.set(Some(new_id));
                        }
                        boudary
                            .waiting_on
                            .borrow_mut()
                            .extend(self.collected_leaves.drain(..));

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
                                // Since this is a boundary, use it as a placeholder
                                let new_id =
                                    self.next_element(template, template.template.node_paths[idx]);
                                component.placeholder.set(Some(new_id));
                                self.scopes[scope.0].placeholder.set(Some(new_id));
                                mutations.push(AssignId {
                                    id: new_id,
                                    path: &template.template.node_paths[idx][1..],
                                });

                                // Now connect everything to the boundary
                                let boundary_mut = boundary;
                                let split_off = mutations.split_off(mutations_to_this_point);
                                let split_off: Vec<Mutation> =
                                    unsafe { std::mem::transmute(split_off) };

                                if boundary_mut.placeholder.get().is_none() {
                                    boundary_mut.placeholder.set(Some(new_id));
                                }

                                // In the generated edits, we want to pick off from where we left off.
                                boundary_mut.mutations.borrow_mut().edits.extend(split_off);

                                boundary_mut
                                    .waiting_on
                                    .borrow_mut()
                                    .extend(self.collected_leaves.drain(..));

                                created = 0;
                            }
                        }

                        // handle any waiting on futures accumulated by async calls down the tree
                        // if this is a boundary, we split off the tree
                        created
                    }
                }
            }

            DynamicNode::Fragment(frag) => {
                // Todo: if there's no children create a placeholder instead ?
                frag.nodes
                    .iter()
                    .fold(0, |acc, child| acc + self.create(mutations, child))
            }

            DynamicNode::Placeholder(slot) => {
                let id = self.next_element(template, template.template.node_paths[idx]);
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
