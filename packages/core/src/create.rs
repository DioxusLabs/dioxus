use crate::VirtualDom;

use crate::any_props::VComponentProps;
use crate::arena::ElementArena;
use crate::component::Component;
use crate::mutations::Mutation;
use crate::nodes::{
    AttributeLocation, DynamicNode, DynamicNodeKind, Template, TemplateId, TemplateNode,
};
use crate::scopes::Scope;
use crate::{
    any_props::AnyProps,
    arena::ElementId,
    bump_frame::BumpFrame,
    nodes::VTemplate,
    scopes::{ComponentPtr, ScopeId, ScopeState},
};
use slab::Slab;

impl VirtualDom {
    /// Create this template and write its mutations
    pub fn create<'a>(
        &mut self,
        mutations: &mut Vec<Mutation<'a>>,
        template: &'a VTemplate<'a>,
    ) -> usize {
        // The best renderers will have tempaltes prehydrated
        // Just in case, let's create the template using instructions anyways
        if !self.templates.contains_key(&template.template.id) {
            self.create_static_template(mutations, template.template);
        }

        // Walk the roots backwards, creating nodes and assigning IDs
        // todo: adjust dynamic nodes to be in the order of roots and then leaves (ie BFS)
        let mut dynamic_attrs = template.dynamic_attrs.iter().peekable();
        let mut dynamic_nodes = template.dynamic_nodes.iter().peekable();

        let mut on_stack = 0;
        for (root_idx, root) in template.template.roots.iter().enumerate() {
            on_stack += match root {
                TemplateNode::Dynamic(id) => {
                    self.create_dynamic_node(mutations, template, &template.dynamic_nodes[*id])
                }
                TemplateNode::DynamicText { .. }
                | TemplateNode::Element { .. }
                | TemplateNode::Text(_) => 1,
            };

            // we're on top of a node that has a dynamic attribute for a descendant
            // Set that attribute now before the stack gets in a weird state
            while let Some(loc) = dynamic_attrs.next_if(|a| a.path[0] == root_idx as u8) {
                // Attach all the elementIDs to the nodes with dynamic content
                let id = self.elements.next();
                mutations.push(Mutation::AssignId {
                    path: &loc.path[1..],
                    id,
                });

                loc.mounted_element.set(id);

                for attr in loc.attrs {
                    mutations.push(Mutation::SetAttribute {
                        name: attr.name,
                        value: attr.value,
                        id,
                    });
                }
            }

            // We're on top of a node that has a dynamic child for a descndent
            // Might as well set it now while we can
            while let Some(node) = dynamic_nodes.next_if(|f| f.path[0] == root_idx as u8) {
                self.create_dynamic_node(mutations, template, node);
            }
        }

        on_stack
    }

    fn create_static_template(&mut self, mutations: &mut Vec<Mutation>, template: &Template) {
        todo!("load template")
    }

    fn create_dynamic_node<'a>(
        &mut self,
        mutations: &mut Vec<Mutation<'a>>,
        template: &VTemplate<'a>,
        node: &'a DynamicNode<'a>,
    ) -> usize {
        match &node.kind {
            DynamicNodeKind::Text { id, value } => {
                let new_id = self.elements.next();
                id.set(new_id);
                mutations.push(Mutation::HydrateText {
                    id: new_id,
                    path: &node.path[1..],
                    value,
                });

                1
            }
            DynamicNodeKind::Component { props, fn_ptr, .. } => {
                let id = self.new_scope(*fn_ptr, None, ElementId(0), *props);

                let template = self.run_scope(id);

                todo!("create component has bad data");
            }
            DynamicNodeKind::Fragment { children } => children
                .iter()
                .fold(0, |acc, child| acc + self.create(mutations, child)),
        }
    }
}
