//! When hydrating streaming components:
//! 1. Just hydrate the template on the outside
//! 2. As we render the virtual dom initially, keep track of the server ids of the suspense boundaries
//! 3. Register a callback for dx_hydrate(id, data) that takes some new data, reruns the suspense boundary with that new data and then rehydrates the node

use super::{session, HydrationSession, SuspenseMessage};
use crate::dom::WebsysDom;
use dioxus_core::{
    AttributeValue, DynamicNode, ElementId, ScopeId, ScopeState, SuspenseBoundaryProps,
    SuspenseContext, TemplateNode, VNode, VirtualDom,
};
use dioxus_fullstack_core::HydrationContext;
use futures_channel::mpsc::UnboundedReceiver;
use std::fmt::Write;
use RehydrationError::*;

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

#[derive(Default)]
pub(crate) struct HydrationOutputs {
    pub(super) ids: Vec<u32>,
    #[cfg_attr(not(feature = "mounted"), allow(dead_code))]
    pub(super) to_mount: Vec<ElementId>,
}

/// Apply the success-path finalize: hand the matched ids + DOM nodes off to
/// the JS interpreter and dispatch any deferred onmounted events.
pub(crate) fn finalize_hydrate(
    websys: &mut WebsysDom,
    outputs: HydrationOutputs,
    under: Vec<web_sys::Node>,
) {
    websys.interpreter.base().hydrate(outputs.ids, under);

    #[cfg(feature = "mounted")]
    for id in outputs.to_mount {
        websys.send_mount_event(id);
    }
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
/// Diagram: <https://excalidraw.com/#json=4NxmW90g0207Y62lESxfF,vP_Yn6j7k23utq2HZIsuiw>
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
    fn add_suspense_boundary(&mut self, id: ScopeId) {
        match self.current_path.as_slice() {
            // This is a root node, add the new node
            [] => {
                self.children.push(SuspenseHydrationIdsNode::new(id));
            }
            // This isn't a root node, traverse into children and add the new node
            [first_index, rest @ ..] => {
                let child_node = self.children[*first_index as usize]
                    .traverse_mut(rest)
                    .unwrap();
                child_node.children.push(SuspenseHydrationIdsNode::new(id));
            }
        }
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
            #[cfg(debug_assertions)]
            debug_types,
            #[cfg(debug_assertions)]
            debug_locations,
        } = message;
        let mut validation = session();

        // Before we start rehydrating the suspense boundary we need to check that the suspense boundary exists. It may have been removed on the client.
        let resolved_suspense_id = path_to_resolved_suspense_id(&suspense_path);
        let resolved_suspense_element = self
            .document
            .get_element_by_id(&resolved_suspense_id)
            .ok_or(RehydrationError::ElementNotFound)?;

        // First convert the dom id into a scope id based on the discovery order of the suspense boundaries.
        // This may fail if the id is not parsable, or if the suspense boundary was removed after partial hydration on the client.
        let id = self
            .suspense_hydration_ids
            .get_suspense_boundary(&suspense_path)
            .ok_or(RehydrationError::SuspenseHydrationIdNotFound)?;

        // Push the new nodes onto the stack
        let children = child_nodes(&resolved_suspense_element);
        for node in &children {
            self.interpreter.base().push_root(node.clone());
        }

        #[cfg(not(debug_assertions))]
        let debug_types = None;
        #[cfg(not(debug_assertions))]
        let debug_locations = None;

        let server_data = HydrationContext::from_serialized(&data, debug_types, debug_locations);
        // If the server serialized an error into the suspense boundary, throw it on the client so that it bubbles up to the nearest error boundary
        if let Some(error) = server_data.error_entry().get().ok().flatten() {
            dom.in_runtime(|| dom.runtime().throw_error(id, error));
        }
        server_data.in_context(|| {
            // rerun the scope with the new data
            SuspenseBoundaryProps::resolve_suspense(
                id,
                dom,
                self,
                |to| {
                    // Switch to only writing templates
                    to.skip_mutations = true;
                },
                children.len(),
            );
            self.skip_mutations = false;
        });

        // Flush the mutations that will swap the placeholder nodes with the resolved nodes
        self.flush_edits();

        // Streaming recovery removes this suspense container, so capture its insertion anchor now while it is still mounted.
        let recovery = validation.streaming_recovery(&resolved_suspense_element);

        // Remove the streaming div
        resolved_suspense_element.remove();

        let Some(root_scope) = dom.get_scope(id) else {
            // The scope was removed on the client. resolve_suspense bailed before
            // consuming the pushed roots, so unwind the JS stack manually.
            for _ in 0..children.len() {
                self.interpreter.pop_root();
            }
            return Ok(());
        };
        let root_scope_id = root_scope.id();

        // As we hydrate the suspense boundary, set the current path to the path of the suspense boundary
        self.suspense_hydration_ids
            .current_path
            .clone_from(&suspense_path);
        // resolve_suspense already consumed the pushed roots via replace_node_with,
        // so the JS stack is balanced when start_hydration_at_scope runs.
        self.start_hydration_at_scope(root_scope_id, dom, children, &mut validation, recovery)?;

        Ok(())
    }

    fn start_hydration_at_scope<S: HydrationSession>(
        &mut self,
        scope_id: ScopeId,
        dom: &mut VirtualDom,
        under: Vec<web_sys::Node>,
        validation: &mut S,
        recovery: S::RecoveryAnchor,
    ) -> Result<(), RehydrationError> {
        let suspense_path = if scope_id == dom.base_scope().id() {
            None
        } else {
            Some(self.suspense_hydration_ids.current_path.clone())
        };
        validation.run_scope(
            self,
            dom,
            scope_id,
            under,
            recovery,
            suspense_path,
            |websys, dom, validation| {
                let mut outputs = HydrationOutputs::default();
                let scope = dom.get_scope(scope_id).ok_or(VNodeNotInitialized)?;
                websys.rehydrate_scope(scope, dom, &mut outputs, validation)?;
                Ok(outputs)
            },
        )
    }

    pub fn rehydrate(
        &mut self,
        vdom: &mut VirtualDom,
    ) -> Result<UnboundedReceiver<SuspenseMessage>, RehydrationError> {
        let mut validation = session();

        let (mut tx, rx) = futures_channel::mpsc::unbounded();
        let closure =
            move |path: Vec<u32>,
                  data: js_sys::Uint8Array,
                  #[allow(unused)] debug_types: Option<Vec<String>>,
                  #[allow(unused)] debug_locations: Option<Vec<String>>| {
                let data = data.to_vec();
                _ = tx.start_send(SuspenseMessage {
                    suspense_path: path,
                    data,
                    #[cfg(debug_assertions)]
                    debug_types,
                    #[cfg(debug_assertions)]
                    debug_locations,
                });
            };
        let closure = wasm_bindgen::closure::Closure::new(closure);
        dioxus_interpreter_js::minimal_bindings::register_rehydrate_chunk_for_streaming_debug(
            &closure,
        );
        closure.forget();

        // Rehydrate the root scope that was rendered on the server. We will likely run into suspense boundaries.
        // Any suspense boundaries we run into are stored for hydration later.
        let recovery = validation.root_recovery();
        self.start_hydration_at_scope(
            vdom.base_scope().id(),
            vdom,
            vec![self.root.clone()],
            &mut validation,
            recovery,
        )?;

        Ok(rx)
    }

    fn rehydrate_scope(
        &mut self,
        scope: &ScopeState,
        dom: &VirtualDom,
        outputs: &mut HydrationOutputs,
        validation: &mut impl HydrationSession,
    ) -> Result<(), RehydrationError> {
        // If this scope is a suspense boundary that is pending, add it to the list of pending suspense boundaries
        if let Some(suspense) =
            SuspenseContext::downcast_suspense_boundary_from_scope(&dom.runtime(), scope.id())
        {
            if suspense.has_suspended_tasks() {
                self.suspense_hydration_ids
                    .add_suspense_boundary(scope.id());
            }
        }

        self.rehydrate_vnode(dom, scope.root_node(), outputs, validation)
    }

    fn rehydrate_vnode(
        &mut self,
        dom: &VirtualDom,
        vnode: &VNode,
        outputs: &mut HydrationOutputs,
        validation: &mut impl HydrationSession,
    ) -> Result<(), RehydrationError> {
        for (i, root) in vnode.template.roots.iter().enumerate() {
            self.rehydrate_template_node(
                dom,
                vnode,
                root,
                outputs,
                Some(vnode.mounted_root(i, dom).ok_or(VNodeNotInitialized)?),
                validation,
            )?;
        }
        Ok(())
    }

    fn rehydrate_template_node(
        &mut self,
        dom: &VirtualDom,
        vnode: &VNode,
        node: &TemplateNode,
        outputs: &mut HydrationOutputs,
        root_id: Option<ElementId>,
        validation: &mut impl HydrationSession,
    ) -> Result<(), RehydrationError> {
        match node {
            TemplateNode::Element {
                attrs, children, ..
            } => validation.element(dom, vnode, node, |validation| {
                let mut mounted_id = root_id;
                for attr in *attrs {
                    if let dioxus_core::TemplateAttribute::Dynamic { id } = attr {
                        let attributes = &*vnode.dynamic_attrs[*id];
                        let id = vnode
                            .mounted_dynamic_attribute(*id, dom)
                            .ok_or(VNodeNotInitialized)?;
                        // We always need to hydrate the node even if the attributes are empty so we have
                        // a mount for the node later. This could be spread attributes that are currently empty,
                        // but will be filled later
                        mounted_id = Some(id);
                        for attribute in attributes {
                            let value = &attribute.value;
                            if let AttributeValue::Listener(_) = value {
                                if attribute.name == "onmounted" {
                                    outputs.to_mount.push(id);
                                }
                            }
                        }
                    }
                }
                if let Some(id) = mounted_id {
                    outputs.ids.push(id.0 as u32);
                }
                for child in *children {
                    self.rehydrate_template_node(dom, vnode, child, outputs, None, validation)?;
                }
                Ok::<(), RehydrationError>(())
            })?,
            TemplateNode::Dynamic { id } => self.rehydrate_dynamic_node(
                dom,
                &vnode.dynamic_nodes[*id],
                *id,
                vnode,
                outputs,
                validation,
            )?,
            TemplateNode::Text { text } => validation.text(text, |_| {
                if let Some(id) = root_id {
                    outputs.ids.push(id.0 as u32);
                }
                Ok::<(), RehydrationError>(())
            })?,
        }
        Ok(())
    }

    fn rehydrate_dynamic_node(
        &mut self,
        dom: &VirtualDom,
        dynamic: &DynamicNode,
        dynamic_node_index: usize,
        vnode: &VNode,
        outputs: &mut HydrationOutputs,
        validation: &mut impl HydrationSession,
    ) -> Result<(), RehydrationError> {
        match dynamic {
            dioxus_core::DynamicNode::Text(text) => validation.text(&text.value, |_| {
                outputs.ids.push(
                    vnode
                        .mounted_dynamic_node(dynamic_node_index, dom)
                        .ok_or(VNodeNotInitialized)?
                        .0 as u32,
                );
                Ok::<(), RehydrationError>(())
            })?,
            dioxus_core::DynamicNode::Placeholder(_) => validation.placeholder(|_| {
                outputs.ids.push(
                    vnode
                        .mounted_dynamic_node(dynamic_node_index, dom)
                        .ok_or(VNodeNotInitialized)?
                        .0 as u32,
                );
                Ok::<(), RehydrationError>(())
            })?,
            dioxus_core::DynamicNode::Component(comp) => {
                validation.component(comp.name, |validation| {
                    let scope = comp
                        .mounted_scope(dynamic_node_index, vnode, dom)
                        .ok_or(VNodeNotInitialized)?;
                    self.rehydrate_scope(scope, dom, outputs, validation)
                })?
            }
            dioxus_core::DynamicNode::Fragment(fragment) => {
                for vnode in fragment {
                    self.rehydrate_vnode(dom, vnode, outputs, validation)?;
                }
            }
        }
        Ok(())
    }
}

/// Collect the direct children of a node into a Vec, snapshotting the sibling
/// chain at call time so callers can mutate the DOM during iteration.
pub(crate) fn child_nodes(node: &web_sys::Node) -> Vec<web_sys::Node> {
    let mut children = Vec::new();
    let mut child = node.first_child();
    while let Some(node) = child {
        children.push(node.clone());
        child = node.next_sibling();
    }
    children
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

fn path_to_resolved_suspense_id(path: &[u32]) -> String {
    let mut resolved_suspense_id_formatted = String::from("ds-");
    write_comma_separated(path, &mut resolved_suspense_id_formatted);
    resolved_suspense_id_formatted.push_str("-r");
    resolved_suspense_id_formatted
}
