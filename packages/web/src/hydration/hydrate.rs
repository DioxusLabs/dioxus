//! When hydrating streaming components:
//! 1. Just hydrate the template on the outside
//! 2. As we render the virtual dom initially, keep track of the server ids of the suspense boundaries
//! 3. Register a callback for dx_hydrate(id, data) that takes some new data, reruns the suspense boundary with that new data and then rehydrates the node

use crate::dom::WebsysDom;
#[cfg(debug_assertions)]
use dioxus_core::{Attribute, TemplateAttribute};
use dioxus_core::{
    AttributeValue, DynamicNode, ElementId, ScopeId, ScopeState, SuspenseBoundaryProps,
    SuspenseContext, TemplateNode, VNode, VirtualDom,
};
use dioxus_fullstack_core::HydrationContext;
use futures_channel::mpsc::UnboundedReceiver;
use std::fmt::Write;
#[cfg(debug_assertions)]
use wasm_bindgen::JsCast;
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

// ============================================================================
// Hydration Validation Types (debug mode only)
// ============================================================================

/// Information about a hydration mismatch between the expected vdom and actual DOM
#[cfg(debug_assertions)]
#[derive(Debug)]
pub(crate) struct HydrationMismatch {
    /// Description of what we expected from the virtual DOM
    pub expected: String,
    /// Description of what we actually found in the DOM
    pub actual: String,
    /// Component path for debugging (e.g., "App > UserProfile > Avatar")
    pub component_path: String,
    /// The suspense path if inside a streaming boundary
    pub suspense_path: Option<Vec<u32>>,
}

/// Validator that tracks component path and collects hydration mismatches
#[cfg(debug_assertions)]
pub(crate) struct HydrationValidator {
    /// Stack of component names for path tracking
    component_stack: Vec<&'static str>,
    /// Current suspense path (if any)
    suspense_path: Option<Vec<u32>>,
    /// Collected mismatches
    mismatches: Vec<HydrationMismatch>,
    /// Stack of DOM traversers - one per level of recursion
    traverser_stack: Vec<DomTraverser>,
}

#[cfg(debug_assertions)]
impl HydrationValidator {
    pub fn new() -> Self {
        Self {
            component_stack: Vec::new(),
            suspense_path: None,
            mismatches: Vec::new(),
            traverser_stack: Vec::new(),
        }
    }

    pub fn with_suspense_path(suspense_path: Vec<u32>) -> Self {
        Self {
            component_stack: Vec::new(),
            suspense_path: Some(suspense_path),
            mismatches: Vec::new(),
            traverser_stack: Vec::new(),
        }
    }

    /// Initialize traversal with the root nodes
    pub fn init_traversal(&mut self, roots: Vec<web_sys::Node>) {
        self.traverser_stack.clear();
        self.traverser_stack.push(DomTraverser::new(roots));
    }

    /// Get the current DOM node without advancing
    pub fn current_dom_node(&self) -> Option<&web_sys::Node> {
        self.traverser_stack.last()?.current()
    }

    /// Advance to the next sibling at the current level
    pub fn advance(&mut self) {
        if let Some(traverser) = self.traverser_stack.last_mut() {
            traverser.next();
        }
    }

    /// Push into children of the current node
    pub fn push_children(&mut self) {
        if let Some(traverser) = self.traverser_stack.last() {
            let children = traverser.children();
            self.traverser_stack.push(children);
        }
    }

    /// Pop back to the parent level
    pub fn pop_children(&mut self) {
        if self.traverser_stack.len() > 1 {
            self.traverser_stack.pop();
        }
    }

    pub fn push_component(&mut self, name: &'static str) {
        self.component_stack.push(name);
    }

    pub fn pop_component(&mut self) {
        self.component_stack.pop();
    }

    fn component_path(&self) -> String {
        if self.component_stack.is_empty() {
            "<root>".to_string()
        } else {
            self.component_stack.join(" > ")
        }
    }

    /// Validate an element node matches expectations
    pub fn validate_element(
        &mut self,
        dom_node: Option<&web_sys::Node>,
        expected_tag: &'static str,
        expected_namespace: Option<&'static str>,
        static_attrs: &'static [TemplateAttribute],
        dynamic_attrs: &[Box<[Attribute]>],
    ) -> bool {
        let expected_desc = Self::describe_expected_element(expected_tag, expected_namespace);

        let Some(dom_node) = dom_node else {
            self.mismatches.push(HydrationMismatch {
                expected: expected_desc,
                actual: "missing node".to_string(),
                component_path: self.component_path(),
                suspense_path: self.suspense_path.clone(),
            });
            return false;
        };

        // Check if it's an element
        let Some(element) = dom_node.dyn_ref::<web_sys::Element>() else {
            self.mismatches.push(HydrationMismatch {
                expected: expected_desc,
                actual: format!("node type {}", dom_node.node_type()),
                component_path: self.component_path(),
                suspense_path: self.suspense_path.clone(),
            });
            return false;
        };

        // Check tag name
        let actual_tag = element.tag_name();
        if !actual_tag.eq_ignore_ascii_case(expected_tag) {
            self.mismatches.push(HydrationMismatch {
                expected: expected_desc,
                actual: format!("<{}>", actual_tag.to_lowercase()),
                component_path: self.component_path(),
                suspense_path: self.suspense_path.clone(),
            });
            return false;
        }

        // Check namespace for SVG etc.
        let actual_ns = element.namespace_uri();
        if actual_ns.as_deref() != expected_namespace {
            self.mismatches.push(HydrationMismatch {
                expected: expected_desc,
                actual: format!(
                    "<{}> (namespace: {:?})",
                    actual_tag.to_lowercase(),
                    actual_ns
                ),
                component_path: self.component_path(),
                suspense_path: self.suspense_path.clone(),
            });
            return false;
        }

        // Check that expected attributes are present (not values, just presence)
        let missing_attrs = self.find_missing_attrs(element, static_attrs, dynamic_attrs);
        if !missing_attrs.is_empty() {
            tracing::debug!(
                "Hydration: element <{}> missing attributes: {:?} at {}",
                expected_tag,
                missing_attrs,
                self.component_path()
            );
            // Don't treat missing attributes as a hard failure - they might be optional
        }

        true
    }

    fn describe_expected_element(tag: &str, namespace: Option<&str>) -> String {
        match namespace {
            Some(ns) => format!("<{}> (namespace: {})", tag, ns),
            None => format!("<{}>", tag),
        }
    }

    /// Validate a text node matches expectations
    pub fn validate_text(&mut self, dom_node: Option<&web_sys::Node>, expected_content: &str) -> bool {
        let expected_desc = format!("text \"{}\"", Self::truncate(expected_content, 50));

        let Some(dom_node) = dom_node else {
            self.mismatches.push(HydrationMismatch {
                expected: expected_desc,
                actual: "missing node".to_string(),
                component_path: self.component_path(),
                suspense_path: self.suspense_path.clone(),
            });
            return false;
        };

        // Text nodes have node_type 3
        if dom_node.node_type() != web_sys::Node::TEXT_NODE {
            self.mismatches.push(HydrationMismatch {
                expected: expected_desc,
                actual: format!("node type {}", dom_node.node_type()),
                component_path: self.component_path(),
                suspense_path: self.suspense_path.clone(),
            });
            return false;
        }

        let actual_content = dom_node.text_content().unwrap_or_default();

        // Normalize whitespace for comparison
        let expected_trimmed = expected_content.trim();
        let actual_trimmed = actual_content.trim();

        if expected_trimmed != actual_trimmed {
            self.mismatches.push(HydrationMismatch {
                expected: expected_desc,
                actual: format!("text \"{}\"", Self::truncate(&actual_content, 50)),
                component_path: self.component_path(),
                suspense_path: self.suspense_path.clone(),
            });
            return false;
        }

        true
    }

    fn truncate(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...", &s[..max_len])
        }
    }

    /// Validate a placeholder (comment) node
    pub fn validate_placeholder(&mut self, dom_node: Option<&web_sys::Node>) -> bool {
        let Some(dom_node) = dom_node else {
            self.mismatches.push(HydrationMismatch {
                expected: "placeholder (comment node)".to_string(),
                actual: "missing node".to_string(),
                component_path: self.component_path(),
                suspense_path: self.suspense_path.clone(),
            });
            return false;
        };

        // Placeholders should be comment nodes (node_type 8)
        if dom_node.node_type() != web_sys::Node::COMMENT_NODE {
            self.mismatches.push(HydrationMismatch {
                expected: "placeholder (comment node)".to_string(),
                actual: format!("node type {}", dom_node.node_type()),
                component_path: self.component_path(),
                suspense_path: self.suspense_path.clone(),
            });
            return false;
        }

        true
    }

    /// Check if any mismatches were found
    pub fn has_mismatches(&self) -> bool {
        !self.mismatches.is_empty()
    }

    /// Report all collected mismatches via tracing::warn!
    pub fn report_mismatches(&self) {
        for mismatch in &self.mismatches {
            let suspense_info = mismatch
                .suspense_path
                .as_ref()
                .map(|p| format!("\n  Suspense Path: {:?}", p))
                .unwrap_or_default();

            tracing::warn!(
                "[HYDRATION MISMATCH] Component: {}\n  Expected: {}\n  Actual: {}{}\n  The subtree will be cleared and rebuilt.",
                mismatch.component_path,
                mismatch.expected,
                mismatch.actual,
                suspense_info
            );
        }
    }

    /// Take the collected mismatches
    pub fn take_mismatches(&mut self) -> Vec<HydrationMismatch> {
        std::mem::take(&mut self.mismatches)
    }

    fn find_missing_attrs(
        &self,
        element: &web_sys::Element,
        static_attrs: &'static [TemplateAttribute],
        dynamic_attrs: &[Box<[Attribute]>],
    ) -> Vec<String> {
        let mut missing = Vec::new();

        // Check static attributes
        for attr in static_attrs {
            if let TemplateAttribute::Static { name, .. } = attr {
                // Skip internal dioxus attributes and event listeners
                if name.starts_with("data-node") || name.starts_with("on") {
                    continue;
                }
                if !element.has_attribute(name) {
                    missing.push((*name).to_string());
                }
            }
        }

        // Check dynamic attributes
        for attrs in dynamic_attrs {
            for attr in attrs.iter() {
                // Skip event listeners and internal attributes
                if attr.name.starts_with("on") || attr.name.starts_with("data-node") {
                    continue;
                }
                // Skip attributes with no value (they might be conditionally rendered)
                if matches!(attr.value, AttributeValue::None) {
                    continue;
                }
                if !element.has_attribute(attr.name) {
                    missing.push(attr.name.to_string());
                }
            }
        }

        missing
    }
}

/// Helper to traverse DOM nodes in parallel with vdom traversal
#[cfg(debug_assertions)]
pub(crate) struct DomTraverser {
    /// Stack of nodes to process (siblings at current level)
    siblings: Vec<web_sys::Node>,
    /// Current index in siblings
    index: usize,
}

#[cfg(debug_assertions)]
impl DomTraverser {
    pub fn new(roots: Vec<web_sys::Node>) -> Self {
        Self {
            siblings: roots,
            index: 0,
        }
    }

    /// Get the current node without advancing
    pub fn current(&self) -> Option<&web_sys::Node> {
        self.siblings.get(self.index)
    }

    /// Advance to the next sibling and return the previous current
    pub fn next(&mut self) -> Option<&web_sys::Node> {
        let node = self.siblings.get(self.index);
        if node.is_some() {
            self.index += 1;
        }
        node
    }

    /// Create a traverser for the children of the current node
    pub fn children(&self) -> Self {
        let Some(current) = self.current() else {
            return Self::new(Vec::new());
        };

        let mut children = Vec::new();
        let mut child = current.first_child();
        while let Some(node) = child {
            children.push(node.clone());
            child = node.next_sibling();
        }

        Self::new(children)
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

        // In debug mode, initialize the validator with the suspense path for streaming hydration
        #[cfg(debug_assertions)]
        {
            self.hydration_validator = Some(HydrationValidator::with_suspense_path(suspense_path.clone()));
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

        // As we hydrate the suspense boundary, set the current path to the path of the suspense boundary
        self.suspense_hydration_ids
            .current_path
            .clone_from(&suspense_path);
        self.start_hydration_at_scope(root_scope, dom, children)?;

        Ok(())
    }

    fn start_hydration_at_scope(
        &mut self,
        scope: &ScopeState,
        dom: &VirtualDom,
        under: Vec<web_sys::Node>,
    ) -> Result<(), RehydrationError> {
        let mut ids = Vec::new();
        let mut to_mount = Vec::new();

        // In debug mode, initialize the validator for DOM traversal
        #[cfg(debug_assertions)]
        {
            if let Some(validator) = &mut self.hydration_validator {
                validator.init_traversal(under.clone());
            }
        }

        // Recursively rehydrate the nodes under the scope
        self.rehydrate_scope(scope, dom, &mut ids, &mut to_mount)?;

        // In debug mode, check for mismatches and handle recovery
        #[cfg(debug_assertions)]
        {
            if let Some(validator) = &mut self.hydration_validator {
                if validator.has_mismatches() {
                    validator.report_mismatches();

                    // Clear the mismatched DOM nodes and rebuild
                    for node in &under {
                        // Remove all children from the node
                        while let Some(child) = node.first_child() {
                            let _ = node.remove_child(&child);
                        }
                    }

                    // Enable mutations so rebuild will work
                    self.skip_mutations = false;

                    tracing::warn!(
                        "Hydration mismatches detected. DOM has been cleared and will be rebuilt."
                    );

                    // Clear the mismatches after handling
                    validator.take_mismatches();
                }
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
        vdom: &VirtualDom,
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
        self.start_hydration_at_scope(vdom.base_scope(), vdom, vec![self.root.clone()])?;

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
                        validator.validate_element(
                            current_node.as_ref(),
                            tag,
                            *namespace,
                            attrs,
                            &vnode.dynamic_attrs,
                        );
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
                        validator.validate_text(current_node.as_ref(), text);
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
                        validator.validate_text(current_node.as_ref(), &text.value);
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
                        validator.validate_placeholder(current_node.as_ref());
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

// ============================================================================
// Tests for Hydration Validation (debug mode only)
// ============================================================================

#[cfg(all(test, debug_assertions))]
mod validation_tests {
    use super::*;

    #[test]
    fn test_validator_new() {
        let validator = HydrationValidator::new();
        assert!(!validator.has_mismatches());
        assert!(validator.component_stack.is_empty());
        assert!(validator.suspense_path.is_none());
    }

    #[test]
    fn test_validator_with_suspense_path() {
        let validator = HydrationValidator::with_suspense_path(vec![0, 1, 2]);
        assert!(!validator.has_mismatches());
        assert_eq!(validator.suspense_path, Some(vec![0, 1, 2]));
    }

    #[test]
    fn test_component_path_tracking() {
        let mut validator = HydrationValidator::new();

        assert_eq!(validator.component_path(), "<root>");

        validator.push_component("App");
        assert_eq!(validator.component_path(), "App");

        validator.push_component("Header");
        assert_eq!(validator.component_path(), "App > Header");

        validator.push_component("NavLink");
        assert_eq!(validator.component_path(), "App > Header > NavLink");

        validator.pop_component();
        assert_eq!(validator.component_path(), "App > Header");

        validator.pop_component();
        assert_eq!(validator.component_path(), "App");

        validator.pop_component();
        assert_eq!(validator.component_path(), "<root>");
    }

    #[test]
    fn test_validate_element_missing_node() {
        let mut validator = HydrationValidator::new();

        let result = validator.validate_element(
            None,
            "div",
            None,
            &[],
            &[],
        );

        assert!(!result);
        assert!(validator.has_mismatches());
        assert_eq!(validator.mismatches.len(), 1);
        assert_eq!(validator.mismatches[0].actual, "missing node");
        assert_eq!(validator.mismatches[0].expected, "<div>");
    }

    #[test]
    fn test_validate_text_missing_node() {
        let mut validator = HydrationValidator::new();

        let result = validator.validate_text(None, "Hello");

        assert!(!result);
        assert!(validator.has_mismatches());
        assert_eq!(validator.mismatches[0].actual, "missing node");
        assert!(validator.mismatches[0].expected.contains("Hello"));
    }

    #[test]
    fn test_validate_placeholder_missing_node() {
        let mut validator = HydrationValidator::new();

        let result = validator.validate_placeholder(None);

        assert!(!result);
        assert!(validator.has_mismatches());
        assert_eq!(validator.mismatches[0].actual, "missing node");
        assert!(validator.mismatches[0].expected.contains("placeholder"));
    }

    #[test]
    fn test_take_mismatches() {
        let mut validator = HydrationValidator::new();
        validator.validate_element(None, "div", None, &[], &[]);

        assert!(validator.has_mismatches());

        let mismatches = validator.take_mismatches();
        assert_eq!(mismatches.len(), 1);
        assert!(!validator.has_mismatches());
    }

    #[test]
    fn test_dom_traverser_new() {
        let traverser = DomTraverser::new(Vec::new());
        assert!(traverser.current().is_none());
    }

    #[test]
    fn test_mismatch_includes_component_path() {
        let mut validator = HydrationValidator::new();
        validator.push_component("App");
        validator.push_component("UserProfile");

        validator.validate_element(None, "div", None, &[], &[]);

        assert_eq!(validator.mismatches[0].component_path, "App > UserProfile");
    }

    #[test]
    fn test_mismatch_includes_suspense_path() {
        let mut validator = HydrationValidator::with_suspense_path(vec![1, 2, 3]);

        validator.validate_element(None, "div", None, &[], &[]);

        assert_eq!(validator.mismatches[0].suspense_path, Some(vec![1, 2, 3]));
    }
}
