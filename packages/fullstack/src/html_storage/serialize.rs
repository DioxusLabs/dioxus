use dioxus_lib::prelude::dioxus_core::DynamicNode;
use dioxus_lib::prelude::{
    has_context, try_consume_context, ScopeId, SuspenseBoundaryProps, SuspenseContext, VNode,
    VirtualDom,
};
use serde::Serialize;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use super::SerializeContext;

#[allow(unused)]
pub(crate) fn serde_to_writable<T: Serialize>(
    value: &T,
    write_to: &mut impl std::fmt::Write,
) -> Result<(), ciborium::ser::Error<std::fmt::Error>> {
    let mut serialized = Vec::new();
    ciborium::into_writer(value, &mut serialized).unwrap();
    write_to.write_str(STANDARD.encode(serialized).as_str())?;
    Ok(())
}

impl super::HTMLData {
    /// Walks through the suspense boundary in a depth first order and extracts the data from the context API.
    /// We use depth first order instead of relying on the order the hooks are called in because during suspense on the server, the order that futures are run in may be non deterministic.
    pub(crate) fn extract_from_suspense_boundary(vdom: &VirtualDom, scope: ScopeId) -> Self {
        let mut data = Self::default();
        data.take_from_scope(vdom, scope);
        data
    }

    fn take_from_virtual_dom(&mut self, vdom: &VirtualDom) {
        self.take_from_scope(vdom, ScopeId::ROOT)
    }

    fn take_from_scope(&mut self, vdom: &VirtualDom, scope: ScopeId) {
        vdom.in_runtime(|| {
            scope.in_runtime(|| {
                // Grab any serializable server context from this scope
                let context: Option<SerializeContext> = has_context();
                if let Some(context) = context {
                    let borrow = context.data.borrow();
                    let mut data = borrow.data.iter().cloned();
                    self.data.extend(data)
                }
            });
        });

        // then continue to any children
        if let Some(scope) = vdom.get_scope(scope) {
            // If this is a suspense boundary, move into the children first (even if they are suspended) because that will be run first on the client
            if let Some(suspense_boundary) =
                SuspenseContext::downcast_suspense_boundary_from_scope(&vdom.runtime(), scope.id())
            {
                if let Some(node) = suspense_boundary.suspended_nodes() {
                    self.take_from_vnode(vdom, &node);
                }
            }
            if let Some(node) = scope.try_root_node() {
                self.take_from_vnode(vdom, node);
            }
        }
    }

    fn take_from_vnode(&mut self, vdom: &VirtualDom, vnode: &VNode) {
        for (dynamic_node_index, dyn_node) in vnode.dynamic_nodes.iter().enumerate() {
            match dyn_node {
                DynamicNode::Component(comp) => {
                    if let Some(scope) = comp.mounted_scope(dynamic_node_index, vnode, vdom) {
                        self.take_from_scope(vdom, scope.id());
                    }
                }
                DynamicNode::Fragment(nodes) => {
                    for node in nodes {
                        self.take_from_vnode(vdom, node);
                    }
                }
                _ => {}
            }
        }
    }

    #[cfg(feature = "server")]
    /// Encode data as base64. This is intended to be used in the server to send data to the client.
    pub(crate) fn serialized(&self) -> String {
        let mut serialized = Vec::new();
        ciborium::into_writer(&self.data, &mut serialized).unwrap();
        base64::engine::general_purpose::STANDARD.encode(serialized)
    }
}
