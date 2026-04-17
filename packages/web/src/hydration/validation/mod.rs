//! Debug-mode hydration validation.
//!
//! This module is only compiled in debug builds. It validates that the
//! server-rendered DOM matches the client virtual-DOM and, on mismatch, emits a
//! human-readable RSX diff before falling back to a full client rebuild.

mod attrs;
mod diff;
pub(super) mod serialize;

use dioxus_core::{TemplateAttribute, TemplateNode, VNode, VirtualDom};
use wasm_bindgen::JsCast;

use self::{
    attrs::find_attribute_mismatches,
    diff::{normalize_debug_rsx, unified_rsx_diff},
    serialize::{
        dom::{serialize_dom_nodes, serialize_dom_subtree, should_skip_validation_node},
        format_rsx_nodes, missing_node, placeholder_node,
        vdom::serialize_template_subtree,
    },
};

/// Information about a hydration mismatch between the expected vdom and actual DOM
#[derive(Debug)]
pub(crate) struct HydrationMismatch {
    /// Summary of what went wrong at this hydration site
    pub reason: String,
    /// Formatted RSX we expected from the virtual DOM
    pub expected_rsx: String,
    /// Formatted RSX we actually found in the DOM
    pub actual_rsx: String,
    /// Component path for debugging (e.g., "App > UserProfile > Avatar")
    pub component_path: String,
    /// The suspense path if inside a streaming boundary
    pub suspense_path: Option<Vec<u32>>,
}

#[derive(Debug, Clone)]
struct ElementMismatchContext {
    expected_rsx: String,
    actual_dom: Option<web_sys::Node>,
}

/// Helper to traverse DOM nodes in parallel with vdom traversal
pub(crate) struct DomTraverser {
    siblings: Vec<web_sys::Node>,
    index: usize,
}

impl DomTraverser {
    pub fn new(roots: Vec<web_sys::Node>) -> Self {
        Self {
            siblings: roots
                .into_iter()
                .filter(|node| !should_skip_validation_node(node))
                .collect(),
            index: 0,
        }
    }

    /// Get the current node without advancing
    pub fn current(&self) -> Option<&web_sys::Node> {
        self.siblings.get(self.index)
    }

    /// Advance to the next sibling
    pub fn next(&mut self) {
        if self.siblings.get(self.index).is_some() {
            self.index += 1;
        }
    }

    pub fn remaining(&self) -> Vec<web_sys::Node> {
        self.siblings[self.index..].to_vec()
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

#[derive(Debug, Clone, Copy)]
enum ComponentFrame {
    Root,
    User(&'static str),
}

#[derive(Default)]
pub(crate) struct HydrationValidationSession {
    /// Stack of component names for path tracking
    component_stack: Vec<ComponentFrame>,
    /// Collected mismatches
    mismatches: Vec<HydrationMismatch>,
    /// Stack of DOM traversers - one per level of recursion
    traverser_stack: Vec<DomTraverser>,
    /// Stack of expected/actual element contexts for scoping node-level diffs
    element_stack: Vec<ElementMismatchContext>,
}

impl super::HydrationSession for HydrationValidationSession {
    fn run_scope<E, F, P, R>(
        &mut self,
        roots: Vec<web_sys::Node>,
        suspense_path: P,
        expected_rsx: R,
        hydrate: F,
    ) -> Result<bool, E>
    where
        F: FnOnce(&mut Self) -> Result<(), E>,
        P: FnOnce() -> Option<Vec<u32>>,
        R: FnOnce() -> Result<String, E>,
    {
        self.init_traversal(roots);
        self.push_root_marker();
        let result = (|| {
            hydrate(self)?;
            self.report_extra_scope_nodes(expected_rsx()?);
            let has_mismatches = self.has_mismatches();
            if has_mismatches {
                let suspense_path = suspense_path();
                self.report_mismatches(suspense_path.as_deref());
                self.take_mismatches(suspense_path.as_deref());
            }

            Ok(has_mismatches)
        })();
        self.pop_root_marker();
        result
    }

    fn element<E, F>(
        &mut self,
        dom: &VirtualDom,
        vnode: &VNode,
        node: &TemplateNode,
        hydrate: F,
    ) -> Result<(), E>
    where
        F: FnOnce(&mut Self) -> Result<(), E>,
    {
        let TemplateNode::Element {
            tag,
            namespace,
            children,
            attrs,
            ..
        } = node
        else {
            unreachable!("element validation requires an element template node");
        };

        let current_node = self.current_dom_node().cloned();
        let expected_rsx = serialize_template_subtree(dom, vnode, node);
        let dynamic_attrs = attrs
            .iter()
            .filter_map(|attr| {
                let TemplateAttribute::Dynamic { id } = attr else {
                    return None;
                };
                Some(&*vnode.dynamic_attrs[*id])
            })
            .collect::<Vec<_>>();

        self.validate_element(
            current_node.as_ref(),
            tag,
            *namespace,
            attrs,
            &dynamic_attrs,
            &expected_rsx,
        );
        self.push_element_context(expected_rsx, current_node);

        let has_children = !children.is_empty();
        if has_children {
            self.push_children();
        }

        let result = hydrate(self);

        if has_children {
            self.pop_children();
        }
        self.pop_element_context();
        self.advance();

        result
    }

    fn text<E, F>(&mut self, expected_content: &str, hydrate: F) -> Result<(), E>
    where
        F: FnOnce(&mut Self) -> Result<(), E>,
    {
        let current_node = self.current_dom_node().cloned();
        self.validate_text(
            current_node.as_ref(),
            expected_content,
            &format!("{expected_content:?}"),
        );

        let result = hydrate(self);
        self.advance();
        result
    }

    fn placeholder<E, F>(&mut self, hydrate: F) -> Result<(), E>
    where
        F: FnOnce(&mut Self) -> Result<(), E>,
    {
        let current_node = self.current_dom_node().cloned();
        self.validate_placeholder(
            current_node.as_ref(),
            &format_rsx_nodes(vec![placeholder_node()]),
        );

        let result = hydrate(self);
        self.advance();
        result
    }

    fn component<E, F>(&mut self, name: &'static str, hydrate: F) -> Result<(), E>
    where
        F: FnOnce(&mut Self) -> Result<(), E>,
    {
        self.push_component(name);
        let result = hydrate(self);
        self.pop_component();
        result
    }
}

impl HydrationValidationSession {
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
            self.report_extra_child_nodes();
            self.traverser_stack.pop();
        }
    }

    pub fn push_root_marker(&mut self) {
        self.component_stack.push(ComponentFrame::Root);
    }

    pub fn pop_root_marker(&mut self) {
        self.component_stack.pop();
    }

    pub fn push_component(&mut self, name: &'static str) {
        self.component_stack.push(ComponentFrame::User(name));
    }

    pub fn pop_component(&mut self) {
        self.component_stack.pop();
    }

    pub fn push_element_context(
        &mut self,
        expected_rsx: String,
        actual_dom: Option<web_sys::Node>,
    ) {
        self.element_stack.push(ElementMismatchContext {
            expected_rsx: normalize_debug_rsx(&expected_rsx),
            actual_dom,
        });
    }

    pub fn pop_element_context(&mut self) {
        self.element_stack.pop();
    }

    fn report_extra_child_nodes(&mut self) {
        let remaining = self
            .traverser_stack
            .last()
            .map(DomTraverser::remaining)
            .unwrap_or_default();

        if let Some(first) = remaining.first() {
            self.push_node_mismatch(
                format!(
                    "Expected no additional child nodes, found {} extra DOM node(s).",
                    remaining.len()
                ),
                "<extra child nodes>".to_string(),
                Some(first),
            );
        }
    }

    fn report_extra_scope_nodes(&mut self, expected_rsx: String) {
        let remaining = self
            .traverser_stack
            .last()
            .map(DomTraverser::remaining)
            .unwrap_or_default();

        if !remaining.is_empty() {
            self.push_mismatch(
                format!(
                    "Expected no additional root nodes, found {} extra DOM node(s).",
                    remaining.len()
                ),
                expected_rsx,
                serialize_dom_nodes(&remaining),
            );
        }
    }

    /// Validate an element node matches expectations
    pub fn validate_element(
        &mut self,
        dom_node: Option<&web_sys::Node>,
        expected_tag: &'static str,
        expected_namespace: Option<&'static str>,
        static_attrs: &'static [TemplateAttribute],
        dynamic_attrs: &[&[dioxus_core::Attribute]],
        expected_rsx: &str,
    ) {
        let expected_desc = describe_expected_element(expected_tag, expected_namespace);

        let Some(dom_node) = dom_node else {
            self.push_element_mismatch(
                format!("Expected {expected_desc}, found missing node."),
                expected_rsx.to_string(),
                None,
            );
            return;
        };

        if let Some(element) = dom_node.dyn_ref::<web_sys::Element>() {
            let actual_tag = element.tag_name();
            if !actual_tag.eq_ignore_ascii_case(expected_tag) {
                self.push_element_mismatch(
                    format!(
                        "Expected {expected_desc}, found <{}>.",
                        actual_tag.to_lowercase()
                    ),
                    expected_rsx.to_string(),
                    Some(dom_node),
                );
            } else {
                let actual_ns = element.namespace_uri();
                let namespace_matches = match expected_namespace {
                    Some(expected_ns) => actual_ns.as_deref() == Some(expected_ns),
                    None => actual_ns
                        .as_deref()
                        .is_none_or(|ns| ns == "http://www.w3.org/1999/xhtml"),
                };
                if !namespace_matches {
                    self.push_element_mismatch(
                        format!(
                            "Expected {expected_desc}, found <{}> with namespace {:?}.",
                            actual_tag.to_lowercase(),
                            actual_ns
                        ),
                        expected_rsx.to_string(),
                        Some(dom_node),
                    );
                } else {
                    let attr_mismatches =
                        find_attribute_mismatches(element, static_attrs, dynamic_attrs);
                    if attr_mismatches.has_mismatches() {
                        self.push_element_mismatch(
                            format!(
                                "Expected {expected_desc} attributes to match, but {}.",
                                attr_mismatches.describe()
                            ),
                            expected_rsx.to_string(),
                            Some(dom_node),
                        );
                    }
                }
            }
        } else {
            self.push_element_mismatch(
                format!(
                    "Expected {expected_desc}, found node type {}.",
                    dom_node.node_type()
                ),
                expected_rsx.to_string(),
                Some(dom_node),
            );
        }
    }

    /// Validate a text node matches expectations
    pub fn validate_text(
        &mut self,
        dom_node: Option<&web_sys::Node>,
        expected_content: &str,
        expected_rsx: &str,
    ) {
        let expected_desc = format!("text {:?}", truncate(expected_content, 50));

        let Some(dom_node) = dom_node else {
            self.push_node_mismatch(
                format!("Expected {expected_desc}, found missing node."),
                expected_rsx.to_string(),
                None,
            );
            return;
        };

        if dom_node.node_type() != web_sys::Node::TEXT_NODE {
            self.push_node_mismatch(
                format!(
                    "Expected {expected_desc}, found node type {}.",
                    dom_node.node_type()
                ),
                expected_rsx.to_string(),
                Some(dom_node),
            );
        } else {
            let actual_content = dom_node.text_content().unwrap_or_default();

            if expected_content != actual_content {
                self.push_node_mismatch(
                    format!(
                        "Expected {expected_desc}, found text {:?}.",
                        truncate(&actual_content, 50)
                    ),
                    expected_rsx.to_string(),
                    Some(dom_node),
                );
            }
        }
    }

    /// Validate a placeholder (comment) node
    pub fn validate_placeholder(&mut self, dom_node: Option<&web_sys::Node>, expected_rsx: &str) {
        let Some(dom_node) = dom_node else {
            self.push_node_mismatch(
                "Expected placeholder (comment node), found missing node.".to_string(),
                expected_rsx.to_string(),
                None,
            );
            return;
        };

        if dom_node.node_type() != web_sys::Node::COMMENT_NODE {
            self.push_node_mismatch(
                format!(
                    "Expected placeholder (comment node), found node type {}.",
                    dom_node.node_type()
                ),
                expected_rsx.to_string(),
                Some(dom_node),
            );
        }
    }

    /// Check if any mismatches were found
    pub fn has_mismatches(&self) -> bool {
        !self.mismatches.is_empty()
    }

    /// Report all collected mismatches via tracing::warn!
    pub fn report_mismatches(&self, suspense_path: Option<&[u32]>) {
        for mismatch in &self.mismatches {
            let suspense_info = suspense_path
                .map(|p| format!("\n  Suspense Path: {:?}", p))
                .unwrap_or_default();
            let diff = unified_rsx_diff(&mismatch.expected_rsx, &mismatch.actual_rsx)
                .lines()
                .map(|line| format!("    {line}"))
                .collect::<Vec<_>>()
                .join("\n");

            tracing::warn!(
                "[HYDRATION MISMATCH] Component: {}\n  Reason: {}\n  RSX Diff:\n{}{}\n  The subtree will be cleared and rebuilt.",
                mismatch.component_path,
                mismatch.reason,
                diff,
                suspense_info,
            );
        }
    }

    /// Take the collected mismatches
    pub fn take_mismatches(&mut self, suspense_path: Option<&[u32]>) -> Vec<HydrationMismatch> {
        let mut mismatches = std::mem::take(&mut self.mismatches);
        let suspense_path = suspense_path.map(<[u32]>::to_vec);
        for mismatch in &mut mismatches {
            mismatch.suspense_path = suspense_path.clone();
        }
        mismatches
    }

    fn component_path(&self) -> String {
        let start = self
            .component_stack
            .iter()
            .rposition(|frame| matches!(frame, ComponentFrame::Root))
            .map_or(0, |pos| pos + 1);
        let user_components = self.component_stack[start..]
            .iter()
            .filter_map(|frame| match frame {
                ComponentFrame::Root => None,
                ComponentFrame::User(name) => Some(*name),
            })
            .collect::<Vec<_>>();

        if user_components.is_empty() {
            "<root>".to_string()
        } else {
            user_components.join(" > ")
        }
    }

    fn push_element_mismatch(
        &mut self,
        reason: String,
        expected_rsx: String,
        actual_dom: Option<&web_sys::Node>,
    ) {
        let actual_rsx = actual_dom
            .map(serialize_dom_subtree)
            .unwrap_or_else(|| format_rsx_nodes(vec![missing_node()]));
        self.push_mismatch(reason, expected_rsx, actual_rsx);
    }

    fn push_node_mismatch(
        &mut self,
        reason: String,
        expected_node_rsx: String,
        actual_node: Option<&web_sys::Node>,
    ) {
        if let Some(context) = self.element_stack.last() {
            let actual_rsx = context
                .actual_dom
                .as_ref()
                .map(serialize_dom_subtree)
                .unwrap_or_else(|| format_rsx_nodes(vec![missing_node()]));
            self.push_mismatch(reason, context.expected_rsx.clone(), actual_rsx);
        } else {
            let actual_rsx = actual_node
                .map(serialize_dom_subtree)
                .unwrap_or_else(|| format_rsx_nodes(vec![missing_node()]));
            self.push_mismatch(reason, expected_node_rsx, actual_rsx);
        }
    }

    fn push_mismatch(&mut self, reason: String, expected_rsx: String, actual_rsx: String) {
        self.mismatches.push(HydrationMismatch {
            reason,
            expected_rsx: normalize_debug_rsx(&expected_rsx),
            actual_rsx: normalize_debug_rsx(&actual_rsx),
            component_path: self.component_path(),
            suspense_path: None,
        });
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    match s.char_indices().nth(max_len) {
        None => s.to_string(),
        Some((byte_idx, _)) => format!("{}...", &s[..byte_idx]),
    }
}

fn describe_expected_element(tag: &str, namespace: Option<&str>) -> String {
    match namespace {
        Some(ns) => format!("<{}> (namespace: {})", tag, ns),
        None => format!("<{}>", tag),
    }
}
