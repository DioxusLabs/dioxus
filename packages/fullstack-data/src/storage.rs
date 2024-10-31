use base64::Engine;
use serialize::serde_to_writable;
use std::{cell::RefCell, io::Cursor, rc::Rc, sync::atomic::AtomicUsize};

use base64::engine::general_purpose::STANDARD;
use serde::{de::DeserializeOwned, Serialize};

#[derive(Default, Clone)]
pub(crate) struct SerializeContext {
    data: Rc<RefCell<HTMLData>>,
}

impl SerializeContext {
    /// Create a new entry in the data that will be sent to the client without inserting any data. Returns an id that can be used to insert data into the entry once it is ready.
    pub(crate) fn create_entry(&self) -> usize {
        self.data.borrow_mut().create_entry()
    }

    /// Insert data into an entry that was created with [`Self::create_entry`]
    pub(crate) fn insert<T: Serialize>(&self, id: usize, value: &T) {
        self.data.borrow_mut().insert(id, value);
    }

    /// Push resolved data into the serialized server data
    pub(crate) fn push<T: Serialize>(&self, data: &T) {
        self.data.borrow_mut().push(data);
    }
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
#[serde(transparent)]
pub(crate) struct HTMLData {
    pub data: Vec<Option<Vec<u8>>>,
}

impl HTMLData {
    /// Create a new entry in the data that will be sent to the client without inserting any data. Returns an id that can be used to insert data into the entry once it is ready.
    pub(crate) fn create_entry(&mut self) -> usize {
        let id = self.data.len();
        self.data.push(None);
        id
    }

    /// Insert data into an entry that was created with [`Self::create_entry`]
    pub(crate) fn insert<T: Serialize>(&mut self, id: usize, value: &T) {
        let mut serialized = Vec::new();
        ciborium::into_writer(value, &mut serialized).unwrap();
        self.data[id] = Some(serialized);
    }

    /// Push resolved data into the serialized server data
    pub(crate) fn push<T: Serialize>(&mut self, data: &T) {
        let mut serialized = Vec::new();
        ciborium::into_writer(data, &mut serialized).unwrap();
        self.data.push(Some(serialized));
    }

    /// Walks through the suspense boundary in a depth first order and extracts the data from the context API.
    /// We use depth first order instead of relying on the order the hooks are called in because during suspense on the server, the order that futures are run in may be non deterministic.
    pub(crate) fn extract_from_suspense_boundary(vdom: &VirtualDom, scope: ScopeId) -> Self {
        let mut data = Self::default();
        // If there is an error boundary on the suspense boundary, grab the error from the context API
        // and throw it on the client so that it bubbles up to the nearest error boundary
        let mut error = vdom.in_runtime(|| {
            scope
                .consume_context::<ErrorContext>()
                .and_then(|error_context| error_context.errors().first().cloned())
        });
        data.push(&error);
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
