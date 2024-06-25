use crate::dom::WebsysDom;
use crate::set_server_data;
use crate::HTMLDataCursor;
use dioxus_core::prelude::*;
use dioxus_core::AttributeValue;
use dioxus_core::{DynamicNode, ElementId};
use futures_channel::mpsc::UnboundedReceiver;
use RehydrationError::*;

// When hydrating streaming components:
// 1. Just hydrate the template on the outside
// 2. As we render the virtual dom initially, keep track of the server ids of the suspense boundaries
// 3. Register a callback for dx_hydrate(id, data) that takes some new data, reruns the suspense boundary with that new data and then rehydrates the node

#[derive(Debug)]
#[non_exhaustive]
pub(crate) enum RehydrationError {
    /// The client tried to rehydrate a vnode before the dom was built
    VNodeNotInitialized,
    /// The client tried to rehydrate a suspense boundary that was not mounted on the server
    SuspenseHydrationIdNotFound,
    /// The client tried to rehydrate a dom id that was not found on the server
    ElementNotFound,
}

/// Streaming hydration happens in waves. The server assigns suspense hydrations ids based on the order
/// the suspense boundaries are discovered in which should be consistent on the client and server.
///
/// This struct keeps track of the order the suspense boundaries are discovered in on the client so we can map the id in the dom to the scope we need to rehydrate.
///
/// Diagram: https://excalidraw.com/#json=GVECyN5gf03RtYEqVq89a,ejIUIzmECANM7bDN0n4UOg
#[derive(Default)]
pub(crate) struct SuspenseHydrationIds {
    /// A dense mapping from traversal order to the scope id of the suspense boundary
    /// The suspense boundary may be unmounted if the component was removed after partial hydration on the client
    ids: Vec<ScopeId>,
}

impl SuspenseHydrationIds {
    /// Add a suspense boundary to the list of suspense boundaries. This should only be called on the root scope after the first rebuild (which happens on the server) and on suspense boundaries that are resolved from the server.
    /// Running this on a scope that is only created on the client may cause hydration issues.
    fn add_suspense_boundary(&mut self, id: ScopeId) {
        self.ids.push(id);
    }

    /// Get the scope id of the suspense boundary from the id in the dom
    fn get_suspense_boundary(&self, id: u32) -> Option<ScopeId> {
        // Indexes come in groups of two. The first index is the unresolved id, the second is the id of the resolved boundary
        let index = id as usize / 2 - 1;
        self.ids.get(index).copied()
    }
}

impl WebsysDom {
    pub fn rehydrate_streaming(&mut self, (id, data): (u32, Vec<u8>), dom: &mut VirtualDom) {
        if let Err(err) = self.rehydrate_streaming_inner(id, data, dom) {
            tracing::error!("Rehydration failed. {:?}", err);
            tracing::error!("Rebuild DOM into element from scratch");
        }
    }

    fn rehydrate_streaming_inner(
        &mut self,
        dom_id: u32,
        data: Vec<u8>,
        dom: &mut VirtualDom,
    ) -> Result<(), RehydrationError> {
        tracing::trace!(
            "Rehydrating streaming chunk {:?}",
            self.suspense_hydration_ids.ids
        );
        // First convert the dom id into a scope id based on the discovery order of the suspense boundaries.
        // This may fail if the id is not parsable, or if the suspense boundary was removed after partial hydration on the client.
        let id = self
            .suspense_hydration_ids
            .get_suspense_boundary(dom_id)
            .ok_or(RehydrationError::SuspenseHydrationIdNotFound)?;

        set_server_data(HTMLDataCursor::from_serialized(&data));

        // rerun the scope with the new data
        dom.mark_dirty(id);
        self.only_write_templates = true;
        SuspenseBoundaryProps::resolve_suspense(id, dom, self);
        self.only_write_templates = false;

        let Some(root_scope) = dom.get_scope(id) else {
            // If the scope was removed on the client, we may not be able to rehydrate it, but this shouldn't cause an error
            return Ok(());
        };

        let element = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id(&format!("ds-{}", dom_id + 1))
            .ok_or(RehydrationError::ElementNotFound)?;

        self.start_hydration_at_scope(root_scope, dom, element)
    }

    fn start_hydration_at_scope(
        &mut self,
        scope: &ScopeState,
        dom: &VirtualDom,
        element: web_sys::Element,
    ) -> Result<(), RehydrationError> {
        let mut ids = Vec::new();
        let mut to_mount = Vec::new();

        // Recursively rehydrate the nodes under the scope
        self.rehydrate_scope(scope, dom, &mut ids, &mut to_mount)?;

        self.interpreter.base().hydrate(ids, element);

        #[cfg(feature = "mounted")]
        for id in to_mount {
            self.send_mount_event(id);
        }

        Ok(())
    }

    pub fn rehydrate(
        &mut self,
        vdom: &VirtualDom,
    ) -> Result<UnboundedReceiver<(u32, Vec<u8>)>, RehydrationError> {
        let (tx, rx) = futures_channel::mpsc::unbounded();
        let closure = move |id: u32, data: js_sys::Uint8Array| {
            let data = data.to_vec();
            tx.unbounded_send((id, data)).unwrap();
        };
        let closure = wasm_bindgen::closure::Closure::new(closure);
        dioxus_interpreter_js::minimal_bindings::register_rehydrate_chunk_for_streaming(&closure);
        closure.forget();

        // Rehydrate the root scope that was rendered on the server. We will likely run into suspense boundaries.
        // Any suspense boundaries we run into are stored for hydration later.
        self.start_hydration_at_scope(vdom.base_scope(), vdom, self.root.clone())?;

        Ok(rx)
    }

    fn rehydrate_scope(
        &mut self,
        scope: &ScopeState,
        dom: &VirtualDom,
        ids: &mut Vec<u32>,
        to_mount: &mut Vec<ElementId>,
    ) -> Result<(), RehydrationError> {
        // If this scope is a suspense boundary that is pending, add it to the list of pending suspense boundaries
        if let Some(suspense) = SuspenseBoundaryProps::downcast_from_scope(scope) {
            tracing::trace!("suspense: {:?}", suspense);
            if suspense.suspended() {
                self.suspense_hydration_ids
                    .add_suspense_boundary(scope.id());
            }
        }

        self.rehydrate_vnode(dom, scope.root_node(), ids, to_mount)
    }

    fn rehydrate_vnode(
        &mut self,
        dom: &VirtualDom,
        vnode: &VNode,
        ids: &mut Vec<u32>,
        to_mount: &mut Vec<ElementId>,
    ) -> Result<(), RehydrationError> {
        for (i, root) in vnode.template.get().roots.iter().enumerate() {
            self.rehydrate_template_node(
                dom,
                vnode,
                root,
                ids,
                to_mount,
                Some(vnode.mounted_root(i, dom).ok_or(VNodeNotInitialized)?),
            )?;
        }
        Ok(())
    }

    fn rehydrate_template_node(
        &mut self,
        dom: &VirtualDom,
        vnode: &VNode,
        node: &TemplateNode,
        ids: &mut Vec<u32>,
        to_mount: &mut Vec<ElementId>,
        root_id: Option<ElementId>,
    ) -> Result<(), RehydrationError> {
        tracing::trace!("rehydrate template node: {:?}", node);
        match node {
            TemplateNode::Element {
                children, attrs, ..
            } => {
                let mut mounted_id = root_id;
                for attr in *attrs {
                    if let dioxus_core::TemplateAttribute::Dynamic { id } = attr {
                        let attributes = &*vnode.dynamic_attrs[*id];
                        let id = vnode
                            .mounted_dynamic_attribute(*id, dom)
                            .ok_or(VNodeNotInitialized)?;
                        for attribute in attributes {
                            let value = &attribute.value;
                            mounted_id = Some(id);
                            if let AttributeValue::Listener(_) = value {
                                if attribute.name == "onmounted" {
                                    to_mount.push(id);
                                }
                            }
                        }
                    }
                }
                if let Some(id) = mounted_id {
                    ids.push(id.0 as u32);
                }
                if !children.is_empty() {
                    for child in *children {
                        self.rehydrate_template_node(dom, vnode, child, ids, to_mount, None)?;
                    }
                }
            }
            TemplateNode::Dynamic { id } => self.rehydrate_dynamic_node(
                dom,
                &vnode.dynamic_nodes[*id],
                *id,
                vnode,
                ids,
                to_mount,
            )?,
            TemplateNode::Text { .. } => {
                if let Some(id) = root_id {
                    ids.push(id.0 as u32);
                }
            }
        }
        Ok(())
    }

    fn rehydrate_dynamic_node(
        &mut self,
        dom: &VirtualDom,
        dynamic: &DynamicNode,
        dynamic_node_index: usize,
        vnode: &VNode,
        ids: &mut Vec<u32>,
        to_mount: &mut Vec<ElementId>,
    ) -> Result<(), RehydrationError> {
        tracing::trace!("rehydrate dynamic node: {:?}", dynamic);
        match dynamic {
            dioxus_core::DynamicNode::Text(_) | dioxus_core::DynamicNode::Placeholder(_) => {
                ids.push(
                    vnode
                        .mounted_dynamic_node(dynamic_node_index, dom)
                        .ok_or(VNodeNotInitialized)?
                        .0 as u32,
                );
            }
            dioxus_core::DynamicNode::Component(comp) => {
                let scope = comp
                    .mounted_scope(dynamic_node_index, vnode, dom)
                    .ok_or(VNodeNotInitialized)?;
                self.rehydrate_scope(scope, dom, ids, to_mount)?;
            }
            dioxus_core::DynamicNode::Fragment(fragment) => {
                for vnode in fragment {
                    self.rehydrate_vnode(dom, vnode, ids, to_mount)?;
                }
            }
        }
        Ok(())
    }
}
