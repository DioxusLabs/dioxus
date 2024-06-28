//! When hydrating streaming components:
//! 1. Just hydrate the template on the outside
//! 2. As we render the virtual dom initially, keep track of the server ids of the suspense boundaries
//! 3. Register a callback for dx_hydrate(id, data) that takes some new data, reruns the suspense boundary with that new data and then rehydrates the node

use std::fmt::Write;

use crate::dom::WebsysDom;
use crate::with_server_data;
use crate::HTMLDataCursor;
use dioxus_core::prelude::*;
use dioxus_core::AttributeValue;
use dioxus_core::{DynamicNode, ElementId};
use dioxus_interpreter_js::minimal_bindings::dx_swap;
use futures_channel::mpsc::UnboundedReceiver;
use RehydrationError::*;

use super::SuspenseMessage;

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

#[derive(Debug)]
struct SuspenseHydrationIdsNode {
    /// The scope id of the suspense boundary
    scope_id: ScopeId,
    /// Children of this node
    children: Vec<SuspenseHydrationIdsNode>,
}

impl SuspenseHydrationIdsNode {
    fn new(scope_id: ScopeId) -> Self {
        Self {
            scope_id,
            children: Vec::new(),
        }
    }

    fn traverse(&self, path: &[u32]) -> Option<&Self> {
        match path {
            [] => Some(self),
            [id, rest @ ..] => self.children.get(*id as usize)?.traverse(rest),
        }
    }

    fn traverse_mut(&mut self, path: &[u32]) -> Option<&mut Self> {
        match path {
            [] => Some(self),
            [id, rest @ ..] => self.children.get_mut(*id as usize)?.traverse_mut(rest),
        }
    }
}

/// Streaming hydration happens in waves. The server assigns suspense hydrations ids based on the order
/// the suspense boundaries are discovered in which should be consistent on the client and server.
///
/// This struct keeps track of the order the suspense boundaries are discovered in on the client so we can map the id in the dom to the scope we need to rehydrate.
///
/// Diagram: https://excalidraw.com/#json=bY2faksLBmN3_vHhh0LGu,liC0hJ3hqQ1MKM--c031Jg
#[derive(Default, Debug)]
pub(crate) struct SuspenseHydrationIds {
    /// A dense mapping from traversal order to the scope id of the suspense boundary
    /// The suspense boundary may be unmounted if the component was removed after partial hydration on the client
    children: Vec<SuspenseHydrationIdsNode>,
    current_path: Vec<u32>,
}

impl SuspenseHydrationIds {
    /// Add a suspense boundary to the list of suspense boundaries. This should only be called on the root scope after the first rebuild (which happens on the server) and on suspense boundaries that are resolved from the server.
    /// Running this on a scope that is only created on the client may cause hydration issues.
    fn add_suspense_boundary(&mut self, id: ScopeId) -> Vec<u32> {
        let mut new_path = self.current_path.clone();
        match self.current_path.as_slice() {
            // This is a root node, add the new node
            [] => {
                let children_len = self.children.len();
                new_path.push(children_len as u32);
                self.children.push(SuspenseHydrationIdsNode::new(id));
            }
            // This isn't a root node, traverse into children and add the new node
            [first_index, rest @ ..] => {
                let child_node = self.children[*first_index as usize]
                    .traverse_mut(rest)
                    .unwrap();
                let new_index = child_node.children.len();
                child_node.children.push(SuspenseHydrationIdsNode::new(id));
                new_path.push(new_index as u32);
            }
        }

        new_path
    }

    /// Get the scope id of the suspense boundary from the id in the dom
    fn get_suspense_boundary(&self, path: &[u32]) -> Option<ScopeId> {
        let root = self.children.get(path[0] as usize)?;
        root.traverse(&path[1..]).map(|node| node.scope_id)
    }
}

impl WebsysDom {
    pub fn rehydrate_streaming(&mut self, message: SuspenseMessage, dom: &mut VirtualDom) {
        if let Err(err) = self.rehydrate_streaming_inner(message, dom) {
            tracing::error!("Rehydration failed. {:?}", err);
        }
    }

    fn rehydrate_streaming_inner(
        &mut self,
        message: SuspenseMessage,
        dom: &mut VirtualDom,
    ) -> Result<(), RehydrationError> {
        let SuspenseMessage {
            suspense_path,
            data,
        } = message;

        let document = web_sys::window().unwrap().document().unwrap();
        // Before we start rehydrating the suspense boundary we need to check that the suspense boundary exists. It may have been removed on the client.
        let suspense_placeholder_id_formatted = path_to_suspense_placeholder_id(&suspense_path);
        if document
            .get_element_by_id(&suspense_placeholder_id_formatted)
            .is_none()
        {
            // Just remove the suspense hydration nodes and return
            let mut resolved_suspense_id = suspense_placeholder_id_formatted.clone();
            resolved_suspense_id.push_str("-r");
            if let Some(element) = document.get_element_by_id(&resolved_suspense_id) {
                element.remove();
            }
            return Ok(());
        }

        // First convert the dom id into a scope id based on the discovery order of the suspense boundaries.
        // This may fail if the id is not parsable, or if the suspense boundary was removed after partial hydration on the client.
        let id = self
            .suspense_hydration_ids
            .get_suspense_boundary(&suspense_path)
            .ok_or(RehydrationError::SuspenseHydrationIdNotFound)?;

        with_server_data(HTMLDataCursor::from_serialized(&data), || {
            // rerun the scope with the new data
            self.only_write_templates = true;
            SuspenseBoundaryProps::resolve_suspense(id, dom, self);
            self.only_write_templates = false;
        });

        let Some(root_scope) = dom.get_scope(id) else {
            // If the scope was removed on the client, we may not be able to rehydrate it, but this shouldn't cause an error
            return Ok(());
        };

        let element = document
            .get_element_by_id(&format!("{suspense_placeholder_id_formatted}-r"))
            .ok_or(RehydrationError::ElementNotFound)?;

        // As we hydrate the suspense boundary, set the current path to the path of the suspense boundary
        self.suspense_hydration_ids
            .current_path
            .clone_from(&suspense_path);
        self.start_hydration_at_scope(root_scope, dom, element)?;

        // After the node is hydrated, swap it into the visible dom
        dx_swap(&suspense_path);

        Ok(())
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
    ) -> Result<UnboundedReceiver<SuspenseMessage>, RehydrationError> {
        let (mut tx, rx) = futures_channel::mpsc::unbounded();
        let closure = move |path: Vec<u32>, data: js_sys::Uint8Array| {
            let data = data.to_vec();
            _ = tx.start_send(SuspenseMessage {
                suspense_path: path,
                data,
            });
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
            if suspense.suspended() {
                let suspense_placeholder_id = self
                    .suspense_hydration_ids
                    .add_suspense_boundary(scope.id());
                // If this suspense boundary is removed before it is resolved, we need to remove the placeholders in the dom.
                // Removing the placeholders will prevent the server from trying to update the new nodes that took its place
                *suspense.on_remove.borrow_mut() = Some(Box::new(move |_| {
                    let suspense_placeholder_id_formatted =
                        path_to_suspense_placeholder_id(&suspense_placeholder_id);
                    if let Some(element) = web_sys::window()
                        .unwrap()
                        .document()
                        .unwrap()
                        .get_element_by_id(&suspense_placeholder_id_formatted)
                    {
                        element.remove();
                    }
                }));
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

fn write_comma_separated(id: &[u32], into: &mut String) {
    let mut iter = id.iter();
    if let Some(first) = iter.next() {
        write!(into, "{first}").unwrap();
    }
    for id in iter {
        write!(into, ",{id}").unwrap();
    }
}

fn path_to_suspense_placeholder_id(path: &[u32]) -> String {
    let mut suspense_placeholder_id_formatted = String::from("ds-");
    write_comma_separated(path, &mut suspense_placeholder_id_formatted);
    suspense_placeholder_id_formatted
}
