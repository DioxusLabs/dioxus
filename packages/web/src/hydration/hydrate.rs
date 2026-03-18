//! When hydrating streaming components:
//! 1. Just hydrate the template on the outside
//! 2. As we render the virtual dom initially, keep track of the server ids of the suspense boundaries
//! 3. Register a callback for dx_hydrate(id, data) that takes some new data, reruns the suspense boundary with that new data and then rehydrates the node

use crate::dom::WebsysDom;
#[cfg(debug_assertions)]
use super::validation::{
    serialize_template_subtree, placeholder_rsx, rsx_string_literal, HydrationValidator,
};
use dioxus_core::{
    AttributeValue, DynamicNode, ElementId, ScopeId, ScopeState, SuspenseBoundaryProps,
    SuspenseContext, TemplateNode, VNode, VirtualDom,
};
use dioxus_fullstack_core::HydrationContext;
use futures_channel::mpsc::UnboundedReceiver;
use std::fmt::Write;
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

        // In debug mode, initialize the validator with the suspense path for streaming hydration
        #[cfg(debug_assertions)]
        {
            self.hydration_validator = Some(HydrationValidator::with_suspense_path(
                suspense_path.clone(),
            ));
        }

        let document = web_sys::window().unwrap().document().unwrap();
        // Before we start rehydrating the suspense boundary we need to check that the suspense boundary exists. It may have been removed on the client.
        let resolved_suspense_id = path_to_resolved_suspense_id(&suspense_path);
        let resolved_suspense_element = document
            .get_element_by_id(&resolved_suspense_id)
            .ok_or(RehydrationError::ElementNotFound)?;

        // First convert the dom id into a scope id based on the discovery order of the suspense boundaries.
        // This may fail if the id is not parsable, or if the suspense boundary was removed after partial hydration on the client.
        let id = self
            .suspense_hydration_ids
            .get_suspense_boundary(&suspense_path)
            .ok_or(RehydrationError::SuspenseHydrationIdNotFound)?;

        // Push the new nodes onto the stack
        let mut current_child = resolved_suspense_element.first_child();
        let mut children = Vec::new();
        while let Some(node) = current_child {
            children.push(node.clone());
            current_child = node.next_sibling();
            self.interpreter.base().push_root(node);
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

        // Remove the streaming div
        resolved_suspense_element.remove();

        let Some(root_scope) = dom.get_scope(id) else {
            // If the scope was removed on the client, we may not be able to rehydrate it, but this shouldn't cause an error
            return Ok(());
        };
        let root_scope_id = root_scope.id();

        // As we hydrate the suspense boundary, set the current path to the path of the suspense boundary
        self.suspense_hydration_ids
            .current_path
            .clone_from(&suspense_path);
        self.start_hydration_at_scope(root_scope_id, dom, children)?;

        Ok(())
    }

    fn clear_root_container(&mut self) {
        while let Some(child) = self.root.first_child() {
            let _ = self.root.remove_child(&child);
        }
    }

    fn start_hydration_at_scope(
        &mut self,
        scope_id: ScopeId,
        dom: &mut VirtualDom,
        under: Vec<web_sys::Node>,
    ) -> Result<(), RehydrationError> {
        let mut ids = Vec::new();
        let mut to_mount = Vec::new();

        // In debug mode, initialize the validator for DOM traversal
        #[cfg(debug_assertions)]
        {
            if let Some(validator) = &mut self.hydration_validator {
                let validation_roots = if scope_id == dom.base_scope().id() {
                    let mut roots = Vec::new();
                    if let Some(root) = under.first() {
                        let mut child = root.first_child();
                        while let Some(node) = child {
                            roots.push(node.clone());
                            child = node.next_sibling();
                        }
                    }
                    roots
                } else {
                    under.clone()
                };

                validator.init_traversal(validation_roots);
            }
        }

        // Recursively rehydrate the nodes under the scope
        {
            let scope = dom
                .get_scope(scope_id)
                .expect("scope should exist during hydration");
            self.rehydrate_scope(scope, dom, &mut ids, &mut to_mount)?;
        }

        // In debug mode, check for mismatches and handle recovery
        #[cfg(debug_assertions)]
        {
            let has_mismatches = self
                .hydration_validator
                .as_ref()
                .is_some_and(HydrationValidator::has_mismatches);

            if has_mismatches {
                if let Some(validator) = &self.hydration_validator {
                    validator.report_mismatches();
                }

                // Switch back to normal rendering and do a fresh client rebuild from the app root.
                // This is heavier than subtree recovery, but it guarantees we recover to an interactive UI.
                self.skip_mutations = false;
                self.clear_root_container();

                tracing::warn!(
                    "Hydration mismatches detected. Falling back to a full client rebuild."
                );

                dom.rebuild(self);
                self.flush_edits();

                if let Some(validator) = &mut self.hydration_validator {
                    validator.take_mismatches();
                }

                return Ok(());
            }
        }

        self.interpreter.base().hydrate(ids, under);

        #[cfg(feature = "mounted")]
        for id in to_mount {
            self.send_mount_event(id);
        }

        Ok(())
    }

    pub fn rehydrate(
        &mut self,
        vdom: &mut VirtualDom,
    ) -> Result<UnboundedReceiver<SuspenseMessage>, RehydrationError> {
        // In debug mode, initialize the validator for hydration validation
        #[cfg(debug_assertions)]
        {
            self.hydration_validator = Some(HydrationValidator::new());
        }

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
        self.start_hydration_at_scope(vdom.base_scope().id(), vdom, vec![self.root.clone()])?;

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
        if let Some(suspense) =
            SuspenseContext::downcast_suspense_boundary_from_scope(&dom.runtime(), scope.id())
        {
            if suspense.has_suspended_tasks() {
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
        for (i, root) in vnode.template.roots.iter().enumerate() {
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
                tag,
                namespace,
                children,
                attrs,
                ..
            } => {
                // In debug mode, validate the element
                #[cfg(debug_assertions)]
                {
                    if let Some(validator) = &mut self.hydration_validator {
                        let current_node = validator.current_dom_node().cloned();
                        let expected_rsx = serialize_template_subtree(dom, vnode, node);
                        let dynamic_attrs = attrs
                            .iter()
                            .filter_map(|attr| {
                                let dioxus_core::TemplateAttribute::Dynamic { id } = attr else {
                                    return None;
                                };
                                Some(&*vnode.dynamic_attrs[*id])
                            })
                            .collect::<Vec<_>>();
                        validator.validate_element(
                            current_node.as_ref(),
                            tag,
                            *namespace,
                            attrs,
                            &dynamic_attrs,
                            &expected_rsx,
                        );
                        validator.push_element_context(expected_rsx, current_node);
                    }
                }
                // Suppress unused warnings in release mode
                #[cfg(not(debug_assertions))]
                {
                    let _ = tag;
                    let _ = namespace;
                }

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
                    // In debug mode, push into children for validation
                    #[cfg(debug_assertions)]
                    {
                        if let Some(validator) = &mut self.hydration_validator {
                            validator.push_children();
                        }
                    }

                    for child in *children {
                        self.rehydrate_template_node(dom, vnode, child, ids, to_mount, None)?;
                    }

                    // In debug mode, pop back to parent level
                    #[cfg(debug_assertions)]
                    {
                        if let Some(validator) = &mut self.hydration_validator {
                            validator.pop_children();
                        }
                    }
                }

                // In debug mode, advance to the next sibling
                #[cfg(debug_assertions)]
                {
                    if let Some(validator) = &mut self.hydration_validator {
                        validator.pop_element_context();
                        validator.advance();
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
            TemplateNode::Text { text } => {
                // In debug mode, validate the text node
                #[cfg(debug_assertions)]
                {
                    if let Some(validator) = &mut self.hydration_validator {
                        let current_node = validator.current_dom_node().cloned();
                        validator.validate_text(
                            current_node.as_ref(),
                            text,
                            &rsx_string_literal(text),
                        );
                        validator.advance();
                    }
                }
                // Suppress unused warning in release mode
                #[cfg(not(debug_assertions))]
                let _ = text;

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
            dioxus_core::DynamicNode::Text(text) => {
                // In debug mode, validate the dynamic text node
                #[cfg(debug_assertions)]
                {
                    if let Some(validator) = &mut self.hydration_validator {
                        let current_node = validator.current_dom_node().cloned();
                        validator.validate_text(
                            current_node.as_ref(),
                            &text.value,
                            &rsx_string_literal(&text.value),
                        );
                        validator.advance();
                    }
                }
                // Suppress unused warning in release mode
                #[cfg(not(debug_assertions))]
                let _ = text;

                ids.push(
                    vnode
                        .mounted_dynamic_node(dynamic_node_index, dom)
                        .ok_or(VNodeNotInitialized)?
                        .0 as u32,
                );
            }
            dioxus_core::DynamicNode::Placeholder(_) => {
                // In debug mode, validate the placeholder (comment) node
                #[cfg(debug_assertions)]
                {
                    if let Some(validator) = &mut self.hydration_validator {
                        let current_node = validator.current_dom_node().cloned();
                        validator.validate_placeholder(current_node.as_ref(), &placeholder_rsx());
                        validator.advance();
                    }
                }

                ids.push(
                    vnode
                        .mounted_dynamic_node(dynamic_node_index, dom)
                        .ok_or(VNodeNotInitialized)?
                        .0 as u32,
                );
            }
            dioxus_core::DynamicNode::Component(comp) => {
                // In debug mode, track the component path
                #[cfg(debug_assertions)]
                {
                    if let Some(validator) = &mut self.hydration_validator {
                        validator.push_component(comp.name);
                    }
                }

                let scope = comp
                    .mounted_scope(dynamic_node_index, vnode, dom)
                    .ok_or(VNodeNotInitialized)?;
                self.rehydrate_scope(scope, dom, ids, to_mount)?;

                // In debug mode, pop the component from the path
                #[cfg(debug_assertions)]
                {
                    if let Some(validator) = &mut self.hydration_validator {
                        validator.pop_component();
                    }
                }
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

fn path_to_resolved_suspense_id(path: &[u32]) -> String {
    let mut resolved_suspense_id_formatted = String::from("ds-");
    write_comma_separated(path, &mut resolved_suspense_id_formatted);
    resolved_suspense_id_formatted.push_str("-r");
    resolved_suspense_id_formatted
}
