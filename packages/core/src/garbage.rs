use crate::{
    nodes::{DynamicNodeKind, VNode},
    scopes::ScopeId,
    virtualdom::VirtualDom,
};

impl VirtualDom {
    pub fn drop_scope(&mut self, id: ScopeId) {
        let scope = self.scopes.get(id.0).unwrap();

        let root = scope.root_node();
        let root = unsafe { std::mem::transmute(root) };

        self.drop_template(root);
    }

    pub fn drop_template<'a>(&'a mut self, template: &'a VNode<'a>) {
        for node in template.dynamic_nodes.iter() {
            match &node.kind {
                DynamicNodeKind::Text { id, .. } => {}

                DynamicNodeKind::Component { .. } => {
                    todo!()
                }

                DynamicNodeKind::Fragment { children } => {}
            }
        }
    }
}
