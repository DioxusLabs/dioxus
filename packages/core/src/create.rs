use std::pin::Pin;

use crate::factory::{FiberLeaf, RenderReturn};
use crate::innerlude::{Renderer, SuspenseContext};
use crate::mutations::Mutation;
use crate::mutations::Mutation::*;
use crate::nodes::VNode;
use crate::nodes::{DynamicNode, TemplateNode};
use crate::virtual_dom::VirtualDom;
use crate::{AttributeValue, Element, ElementId, TemplateAttribute};
use bumpalo::boxed::Box as BumpBox;
use futures_util::Future;

impl VirtualDom {
    /// Create this template and write its mutations
    pub fn create<'a>(&mut self, mutations: &mut Renderer<'a>, template: &'a VNode<'a>) -> usize {
        // The best renderers will have templates prehydrated
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

        // Walk the roots backwards, creating nodes and assigning IDs
        // todo: adjust dynamic nodes to be in the order of roots and then leaves (ie BFS)
        let mut dynamic_attrs = template.template.attr_paths.iter().enumerate().peekable();
        let mut dynamic_nodes = template.template.node_paths.iter().enumerate().peekable();

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

                TemplateNode::Dynamic(id) => {
                    self.create_dynamic_node(mutations, template, &template.dynamic_nodes[*id], *id)
                }

                TemplateNode::DynamicText { .. } => 1,
            };

            let mut cur_route = None;

            // we're on top of a node that has a dynamic attribute for a descendant
            // Set that attribute now before the stack gets in a weird state
            while let Some((idx, path)) = dynamic_attrs.next_if(|(_, p)| p[0] == root_idx as u8) {
                let attr = &template.dynamic_attrs[idx];

                if cur_route.is_none() {
                    cur_route = Some((self.next_element(template), &path[1..]));
                }

                // Attach all the elementIDs to the nodes with dynamic content
                let (id, path) = cur_route.unwrap();

                mutations.push(AssignId { path, id });
                attr.mounted_element.set(id);

                match attr.value {
                    AttributeValue::Text(value) => {
                        mutations.push(SetAttribute {
                            name: attr.name,
                            value,
                            id,
                        });
                    }

                    AttributeValue::Listener(_) => {
                        //
                    }

                    AttributeValue::Float(_) => todo!(),
                    AttributeValue::Int(_) => todo!(),
                    AttributeValue::Bool(_) => todo!(),
                    AttributeValue::Any(_) => todo!(),

                    // Optional attributes
                    AttributeValue::None => todo!(),
                }
            }

            // We're on top of a node that has a dynamic child for a descendant
            // Skip any node that's a root
            while let Some((idx, path)) =
                dynamic_nodes.next_if(|(_, p)| p.len() > 1 && p[0] == root_idx as u8)
            {
                let node = &template.dynamic_nodes[idx];
                let m = self.create_dynamic_node(mutations, template, node, idx);
                if m > 0 {
                    mutations.push(ReplacePlaceholder {
                        m,
                        path: &path[1..],
                    });
                }
            }
        }

        on_stack
    }

    pub fn create_static_node<'a>(
        &mut self,
        mutations: &mut Vec<Mutation<'a>>,
        template: &'a VNode<'a>,
        node: &'a TemplateNode<'static>,
    ) {
        match *node {
            // Todo: create the children's template
            TemplateNode::Dynamic(_) => mutations.push(CreatePlaceholder { id: ElementId(0) }),
            TemplateNode::Text(value) => mutations.push(CreateText { value }),
            TemplateNode::DynamicText { .. } => mutations.push(CreateText {
                value: "placeholder",
            }),
            TemplateNode::Element {
                attrs,
                children,
                namespace,
                tag,
            } => {
                let id = self.next_element(template);

                mutations.push(CreateElement {
                    name: tag,
                    namespace,
                    id,
                });

                mutations.extend(attrs.into_iter().filter_map(|attr| match attr {
                    TemplateAttribute::Static { name, value, .. } => {
                        Some(SetAttribute { name, value, id })
                    }
                    _ => None,
                }));

                children
                    .into_iter()
                    .for_each(|child| self.create_static_node(mutations, template, child));

                mutations.push(AppendChildren { m: children.len() })
            }
        }
    }

    pub fn create_dynamic_node<'a>(
        &mut self,
        mutations: &mut Renderer<'a>,
        template: &'a VNode<'a>,
        node: &'a DynamicNode<'a>,
        idx: usize,
    ) -> usize {
        match &node {
            DynamicNode::Text { id, value } => {
                let new_id = self.next_element(template);
                id.set(new_id);
                mutations.push(HydrateText {
                    id: new_id,
                    path: &template.template.node_paths[idx][1..],
                    value,
                });

                1
            }

            DynamicNode::Component {
                props, placeholder, ..
            } => {
                let id = self.new_scope(unsafe { std::mem::transmute(props.get()) });
                let render_ret = self.run_scope(id);
                let render_ret: &mut RenderReturn = unsafe { std::mem::transmute(render_ret) };

                // if boundary or subtree, start working on a new stack of mutations

                match render_ret {
                    RenderReturn::Sync(None) | RenderReturn::Async(_) => {
                        let new_id = self.next_element(template);
                        placeholder.set(Some(new_id));
                        self.scopes[id.0].placeholder.set(Some(new_id));
                        mutations.push(AssignId {
                            id: new_id,
                            path: &template.template.node_paths[idx][1..],
                        });
                        0
                    }

                    RenderReturn::Sync(Some(template)) => {
                        let mutations_to_this_point = mutations.len();

                        self.scope_stack.push(id);
                        let mut created = self.create(mutations, template);
                        self.scope_stack.pop();

                        if !self.waiting_on.is_empty() {
                            if let Some(boundary) =
                                self.scopes[id.0].has_context::<SuspenseContext>()
                            {
                                let mut boundary_mut = boundary.borrow_mut();
                                let split_off = mutations.split_off(mutations_to_this_point);

                                let split_off = unsafe { std::mem::transmute(split_off) };

                                println!("SPLIT OFF: {:#?}", split_off);

                                boundary_mut.mutations.mutations = split_off;
                                boundary_mut.waiting_on.extend(self.waiting_on.drain(..));

                                // Since this is a boundary, use it as a placeholder
                                let new_id = self.next_element(template);
                                placeholder.set(Some(new_id));
                                self.scopes[id.0].placeholder.set(Some(new_id));
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

            DynamicNode::Fragment(children) => children
                .iter()
                .fold(0, |acc, child| acc + self.create(mutations, child)),

            DynamicNode::Placeholder(_) => {
                let id = self.next_element(template);
                mutations.push(CreatePlaceholder { id });
                1
            }
        }
    }
}
