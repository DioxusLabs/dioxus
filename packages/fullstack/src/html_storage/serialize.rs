use dioxus_lib::prelude::dioxus_core::DynamicNode;
use dioxus_lib::prelude::{try_consume_context, ScopeId, VNode, VirtualDom};
use serde::Serialize;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use super::SerializeContext;

#[allow(unused)]
fn serde_to_writable<T: Serialize>(
    value: &T,
    write_to: &mut impl std::io::Write,
) -> Result<(), ciborium::ser::Error<std::io::Error>> {
    let mut serialized = Vec::new();
    ciborium::into_writer(value, &mut serialized)?;
    write_to.write_all(STANDARD.encode(serialized).as_bytes())?;
    Ok(())
}

#[cfg(feature = "server")]
/// Encode data into a element. This is inteded to be used in the server to send data to the client.
pub(crate) fn encode_in_element(
    data: &super::HTMLData,
    write_to: &mut impl std::io::Write,
) -> Result<(), ciborium::ser::Error<std::io::Error>> {
    write_to.write_all(
        r#"<meta hidden="true" id="dioxus-storage-data" data-serialized=""#.as_bytes(),
    )?;
    serde_to_writable(&data, write_to)?;
    Ok(write_to.write_all(r#"" />"#.as_bytes())?)
}

impl super::HTMLData {
    /// Walks through the virtual dom in a depth first order and extracts the data from the context API.
    /// We use depth first order instead of relying on the order the hooks are called in because during suspense on the server, the order that futures are run in may be non deterministic.
    pub(crate) fn extract_from_virtual_dom(vdom: &VirtualDom) -> Self {
        let mut data = Self::default();
        data.take_from_virtual_dom(vdom);
        data
    }

    fn take_from_virtual_dom(&mut self, vdom: &VirtualDom) {
        self.take_from_scope(vdom, ScopeId::ROOT)
    }

    fn take_from_scope(&mut self, vdom: &VirtualDom, scope: ScopeId) {
        vdom.in_runtime(|| {
            scope.in_runtime(|| {
                // Insert any context from the parent first
                let context: Option<SerializeContext> = try_consume_context();
                if let Some(context) = context {
                    let mut data = context.data.clone();
                    self.data.extend(data.take().data)
                }
            });
        });

        // then continue to any children
        if let Some(scope) = vdom.get_scope(scope) {
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
}
