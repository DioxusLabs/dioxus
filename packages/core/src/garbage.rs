use crate::{nodes::VNode, scopes::ScopeId, virtual_dom::VirtualDom, DynamicNode, Mutations};

impl<'b> VirtualDom {
    pub fn drop_scope(&mut self, id: ScopeId) {
        // let scope = self.scopes.get(id.0).unwrap();

        // let root = scope.root_node();
        // let root = unsafe { std::mem::transmute(root) };

        // self.drop_template(root, false);
        todo!()
    }

    pub fn drop_template(
        &mut self,
        mutations: &mut Mutations,
        template: &'b VNode<'b>,
        gen_roots: bool,
    ) {
        for node in template.dynamic_nodes.iter() {
            match node {
                DynamicNode::Text { id, .. } => {}

                DynamicNode::Component { .. } => {
                    todo!()
                }

                DynamicNode::Fragment { inner, nodes } => {}
                DynamicNode::Placeholder(_) => todo!(),
                _ => todo!(),
            }
        }
    }
}
