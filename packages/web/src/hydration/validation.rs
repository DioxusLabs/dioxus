//! Debug-mode hydration validation.
//!
//! Everything in this module is gated behind `#[cfg(debug_assertions)]` at the
//! module level (see `mod.rs`).  It validates that the server-rendered DOM
//! matches the client virtual-DOM and, on mismatch, emits a human-readable RSX
//! diff before falling back to a full client rebuild.

use dioxus_autofmt::write_block_out;
use dioxus_core::{
    Attribute, AttributeValue, DynamicNode, TemplateAttribute, TemplateNode, VNode, VirtualDom,
};
use dioxus_rsx::CallBody;
use syn::parse::Parser;
use wasm_bindgen::JsCast;

// ---------------------------------------------------------------------------
// Mismatch type
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Element mismatch context (private)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct ElementMismatchContext {
    expected_rsx: String,
    actual_dom: Option<web_sys::Node>,
}

// ---------------------------------------------------------------------------
// DomTraverser
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// HydrationValidator
// ---------------------------------------------------------------------------

/// Validator that tracks component path and collects hydration mismatches
pub(crate) struct HydrationValidator {
    /// Stack of component names for path tracking
    component_stack: Vec<&'static str>,
    /// Current suspense path (if any)
    suspense_path: Option<Vec<u32>>,
    /// Collected mismatches
    mismatches: Vec<HydrationMismatch>,
    /// Stack of DOM traversers - one per level of recursion
    traverser_stack: Vec<DomTraverser>,
    /// Stack of expected/actual element contexts for scoping node-level diffs
    element_stack: Vec<ElementMismatchContext>,
}

impl HydrationValidator {
    pub fn new() -> Self {
        Self {
            component_stack: Vec::new(),
            suspense_path: None,
            mismatches: Vec::new(),
            traverser_stack: Vec::new(),
            element_stack: Vec::new(),
        }
    }

    pub fn with_suspense_path(suspense_path: Vec<u32>) -> Self {
        Self {
            component_stack: Vec::new(),
            suspense_path: Some(suspense_path),
            mismatches: Vec::new(),
            traverser_stack: Vec::new(),
            element_stack: Vec::new(),
        }
    }

    // -- traversal ----------------------------------------------------------

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

    // -- component path -----------------------------------------------------

    pub fn push_component(&mut self, name: &'static str) {
        self.component_stack.push(name);
    }

    pub fn pop_component(&mut self) {
        self.component_stack.pop();
    }

    // -- element context ----------------------------------------------------

    pub fn push_element_context(
        &mut self,
        expected_rsx: String,
        actual_dom: Option<web_sys::Node>,
    ) {
        self.element_stack.push(ElementMismatchContext {
            expected_rsx: normalize_rsx_block(&expected_rsx),
            actual_dom,
        });
    }

    pub fn pop_element_context(&mut self) {
        self.element_stack.pop();
    }

    // -- validation ---------------------------------------------------------

    /// Validate an element node matches expectations
    pub fn validate_element(
        &mut self,
        dom_node: Option<&web_sys::Node>,
        expected_tag: &'static str,
        expected_namespace: Option<&'static str>,
        static_attrs: &'static [TemplateAttribute],
        dynamic_attrs: &[&[Attribute]],
        expected_rsx: &str,
    ) -> bool {
        let expected_desc = describe_expected_element(expected_tag, expected_namespace);

        let Some(dom_node) = dom_node else {
            self.push_element_mismatch(
                format!("Expected {expected_desc}, found missing node."),
                expected_rsx.to_string(),
                None,
            );
            return false;
        };

        // Check if it's an element
        let Some(element) = dom_node.dyn_ref::<web_sys::Element>() else {
            self.push_element_mismatch(
                format!(
                    "Expected {expected_desc}, found node type {}.",
                    dom_node.node_type()
                ),
                expected_rsx.to_string(),
                Some(dom_node),
            );
            return false;
        };

        // Check tag name
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
            return false;
        }

        // Check namespace for SVG etc.
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
            return false;
        }

        // Check that expected attributes are present (not values, just presence)
        let missing_attrs = find_missing_attrs(element, static_attrs, dynamic_attrs);
        if !missing_attrs.is_empty() {
            let missing_attrs = describe_missing_attrs(&missing_attrs);
            self.push_element_mismatch(
                format!(
                    "Expected {expected_desc} with attributes [{missing_attrs}], but the DOM node is missing them."
                ),
                expected_rsx.to_string(),
                Some(dom_node),
            );
            return false;
        }

        true
    }

    /// Validate a text node matches expectations
    pub fn validate_text(
        &mut self,
        dom_node: Option<&web_sys::Node>,
        expected_content: &str,
        expected_rsx: &str,
    ) -> bool {
        let expected_desc = format!(
            "text {}",
            rsx_string_literal(&truncate(expected_content, 50))
        );

        let Some(dom_node) = dom_node else {
            self.push_node_mismatch(
                format!("Expected {expected_desc}, found missing node."),
                expected_rsx.to_string(),
                None,
            );
            return false;
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
            return false;
        }

        let actual_content = dom_node.text_content().unwrap_or_default();
        let expected_trimmed = expected_content.trim();
        let actual_trimmed = actual_content.trim();

        if expected_trimmed != actual_trimmed {
            self.push_node_mismatch(
                format!(
                    "Expected {expected_desc}, found text {}.",
                    rsx_string_literal(&truncate(&actual_content, 50))
                ),
                expected_rsx.to_string(),
                Some(dom_node),
            );
            return false;
        }

        true
    }

    /// Validate a placeholder (comment) node
    pub fn validate_placeholder(
        &mut self,
        dom_node: Option<&web_sys::Node>,
        expected_rsx: &str,
    ) -> bool {
        let Some(dom_node) = dom_node else {
            self.push_node_mismatch(
                "Expected placeholder (comment node), found missing node.".to_string(),
                expected_rsx.to_string(),
                None,
            );
            return false;
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
            return false;
        }

        true
    }

    // -- mismatch queries ---------------------------------------------------

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
            let diff = indent_block(
                &unified_rsx_diff(&mismatch.expected_rsx, &mismatch.actual_rsx),
                "    ",
            );

            tracing::warn!(
                "[HYDRATION MISMATCH] Component: {}\n  Reason: {}\n  RSX Diff:\n{}{}\n  The subtree will be cleared and rebuilt.",
                mismatch.component_path,
                mismatch.reason,
                diff,
                suspense_info
            );
        }
    }

    /// Take the collected mismatches
    pub fn take_mismatches(&mut self) -> Vec<HydrationMismatch> {
        std::mem::take(&mut self.mismatches)
    }

    // -- private helpers ----------------------------------------------------

    fn component_path(&self) -> String {
        // Strip internal framework components (SuspenseBoundary, ErrorBoundary, etc.)
        // by only showing the path after the last "root" component.
        let user_components: &[&str] = match self
            .component_stack
            .iter()
            .rposition(|name| *name == "root")
        {
            Some(pos) => &self.component_stack[pos + 1..],
            None => &self.component_stack,
        };

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
            .unwrap_or_else(missing_node_rsx);
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
                .unwrap_or_else(missing_node_rsx);
            self.push_mismatch(reason, context.expected_rsx.clone(), actual_rsx);
        } else {
            let actual_rsx = actual_node
                .map(serialize_dom_subtree)
                .unwrap_or_else(missing_node_rsx);
            self.push_mismatch(reason, expected_node_rsx, actual_rsx);
        }
    }

    fn push_mismatch(&mut self, reason: String, expected_rsx: String, actual_rsx: String) {
        self.mismatches.push(HydrationMismatch {
            reason,
            expected_rsx: normalize_rsx_block(&expected_rsx),
            actual_rsx: normalize_rsx_block(&actual_rsx),
            component_path: self.component_path(),
            suspense_path: self.suspense_path.clone(),
        });
    }
}

// ===========================================================================
// VNode → RSX serialization
// ===========================================================================

pub(crate) fn serialize_template_subtree(dom: &VirtualDom, vnode: &VNode, node: &TemplateNode) -> String {
    serialize_items_as_block(&serialize_template_node_items(dom, vnode, node))
}

fn serialize_template_node_items(
    dom: &VirtualDom,
    vnode: &VNode,
    node: &TemplateNode,
) -> Vec<String> {
    match node {
        TemplateNode::Element {
            tag,
            attrs,
            children,
            ..
        } => {
            let mut attributes = serialize_template_attributes(attrs, vnode);
            let mut child_items = Vec::new();
            for child in *children {
                child_items.extend(serialize_template_node_items(dom, vnode, child));
            }
            attributes.sort();
            vec![render_element_rsx(tag, attributes, child_items)]
        }
        TemplateNode::Text { text } => vec![rsx_string_literal(text)],
        TemplateNode::Dynamic { id } => serialize_dynamic_node_items(dom, vnode, *id),
    }
}

fn serialize_dynamic_node_items(dom: &VirtualDom, vnode: &VNode, dynamic_id: usize) -> Vec<String> {
    match &vnode.dynamic_nodes[dynamic_id] {
        DynamicNode::Text(text) => vec![rsx_string_literal(&text.value)],
        DynamicNode::Placeholder(_) => vec![placeholder_rsx()],
        DynamicNode::Fragment(fragment) => fragment
            .iter()
            .flat_map(|fragment_vnode| serialize_vnode_items(dom, fragment_vnode))
            .collect(),
        DynamicNode::Component(comp) => comp
            .mounted_scope(dynamic_id, vnode, dom)
            .map(|scope| serialize_vnode_items(dom, scope.root_node()))
            .unwrap_or_else(|| vec![format!("{} {{}}", rsx_name(comp.name))]),
    }
}

fn serialize_vnode_items(dom: &VirtualDom, vnode: &VNode) -> Vec<String> {
    vnode
        .template
        .roots
        .iter()
        .flat_map(|root| serialize_template_node_items(dom, vnode, root))
        .collect()
}

fn serialize_template_attributes(
    attrs: &'static [TemplateAttribute],
    vnode: &VNode,
) -> Vec<String> {
    let mut rendered = Vec::new();

    for attr in attrs {
        match attr {
            TemplateAttribute::Static { name, value, .. } => {
                if let Some(rendered_attr) = render_static_template_attribute(name, value) {
                    rendered.push(rendered_attr);
                }
            }
            TemplateAttribute::Dynamic { id } => {
                let mut dynamic_attrs: Vec<_> = vnode.dynamic_attrs[*id]
                    .iter()
                    .filter_map(render_dynamic_template_attribute)
                    .collect();
                dynamic_attrs.sort();
                rendered.extend(dynamic_attrs);
            }
        }
    }

    rendered
}

fn render_static_template_attribute(name: &str, value: &str) -> Option<String> {
    if is_internal_attribute_name(name) {
        return None;
    }

    let rendered_value = if is_boolean_html_attribute(name) && (value.is_empty() || value == "true")
    {
        "true".to_string()
    } else {
        rsx_string_literal(value)
    };

    Some(render_attribute(name, rendered_value))
}

fn render_dynamic_template_attribute(attr: &Attribute) -> Option<String> {
    if is_internal_attribute_name(attr.name)
        || matches!(
            attr.value,
            AttributeValue::Listener(_) | AttributeValue::None
        )
    {
        return None;
    }

    let rendered_value = match &attr.value {
        AttributeValue::Text(value) => rsx_string_literal(value),
        AttributeValue::Float(value) if value.is_finite() => value.to_string(),
        AttributeValue::Float(_) => rsx_string_literal("<non-finite-float>"),
        AttributeValue::Int(value) => value.to_string(),
        AttributeValue::Bool(value) => value.to_string(),
        AttributeValue::Any(_) => rsx_string_literal("<any>"),
        AttributeValue::Listener(_) | AttributeValue::None => return None,
    };

    Some(render_attribute(attr.name, rendered_value))
}

// ===========================================================================
// DOM → RSX serialization
// ===========================================================================

fn serialize_dom_subtree(node: &web_sys::Node) -> String {
    serialize_items_as_block(&serialize_dom_node_items(node))
}

fn serialize_dom_node_items(node: &web_sys::Node) -> Vec<String> {
    if should_skip_validation_node(node) {
        return Vec::new();
    }

    match node.node_type() {
        web_sys::Node::ELEMENT_NODE => {
            let Some(element) = node.dyn_ref::<web_sys::Element>() else {
                return vec![missing_node_rsx()];
            };

            let mut attrs = serialize_dom_attributes(element);
            let mut children = Vec::new();
            let mut child = node.first_child();
            while let Some(current) = child {
                children.extend(serialize_dom_node_items(&current));
                child = current.next_sibling();
            }

            attrs.sort();
            vec![render_element_rsx(
                &element.tag_name().to_lowercase(),
                attrs,
                children,
            )]
        }
        web_sys::Node::TEXT_NODE => {
            vec![rsx_string_literal(&node.text_content().unwrap_or_default())]
        }
        web_sys::Node::COMMENT_NODE => {
            let comment = node.text_content().unwrap_or_default();
            if is_placeholder_comment(&comment) {
                vec![placeholder_rsx()]
            } else {
                vec![rsx_string_literal(&format!("<!--{}-->", comment.trim()))]
            }
        }
        _ => vec![rsx_string_literal(&format!(
            "<node type {}>",
            node.node_type()
        ))],
    }
}

fn serialize_dom_attributes(element: &web_sys::Element) -> Vec<String> {
    let mut rendered = Vec::new();
    let names = element.get_attribute_names();

    for idx in 0..names.length() {
        let Some(name) = names.get(idx).as_string() else {
            continue;
        };
        if is_internal_attribute_name(&name) {
            continue;
        }
        let value = element.get_attribute(&name).unwrap_or_default();
        let rendered_value = if is_boolean_html_attribute(&name) && value.is_empty() {
            "true".to_string()
        } else {
            rsx_string_literal(&value)
        };
        rendered.push(render_attribute(&name, rendered_value));
    }

    rendered
}

// ===========================================================================
// RSX rendering helpers
// ===========================================================================

fn render_element_rsx(tag: &str, attributes: Vec<String>, children: Vec<String>) -> String {
    if attributes.is_empty() && children.is_empty() {
        return format!("{} {{}}", rsx_name(tag));
    }

    let mut out = format!("{} {{", rsx_name(tag));
    for attribute in attributes {
        out.push('\n');
        out.push_str(&attribute);
        out.push(',');
    }
    for child in children {
        out.push('\n');
        out.push_str(&child);
    }
    out.push('\n');
    out.push('}');
    out
}

fn render_attribute(name: &str, value: String) -> String {
    format!("{}: {}", rsx_name(name), value)
}

fn serialize_items_as_block(items: &[String]) -> String {
    match items {
        [] => missing_node_rsx(),
        _ => items.join("\n"),
    }
}

// ===========================================================================
// RSX formatting / diffing
// ===========================================================================

pub(crate) fn normalize_rsx_block(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return trimmed.to_string();
    }

    CallBody::parse_strict
        .parse_str(trimmed)
        .ok()
        .and_then(|body| write_block_out(&body))
        .unwrap_or_else(|| trimmed.to_string())
}

fn unified_rsx_diff(expected: &str, actual: &str) -> String {
    let expected_lines: Vec<&str> = expected.lines().collect();
    let actual_lines: Vec<&str> = actual.lines().collect();
    let mut lcs = vec![vec![0usize; actual_lines.len() + 1]; expected_lines.len() + 1];

    for i in (0..expected_lines.len()).rev() {
        for j in (0..actual_lines.len()).rev() {
            lcs[i][j] = if expected_lines[i] == actual_lines[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }

    let mut i = 0;
    let mut j = 0;
    let mut diff_lines = Vec::new();

    while i < expected_lines.len() && j < actual_lines.len() {
        if expected_lines[i] == actual_lines[j] {
            diff_lines.push(format!(" {}", expected_lines[i]));
            i += 1;
            j += 1;
        } else if lcs[i + 1][j] >= lcs[i][j + 1] {
            diff_lines.push(format!("-{}", expected_lines[i]));
            i += 1;
        } else {
            diff_lines.push(format!("+{}", actual_lines[j]));
            j += 1;
        }
    }

    while i < expected_lines.len() {
        diff_lines.push(format!("-{}", expected_lines[i]));
        i += 1;
    }

    while j < actual_lines.len() {
        diff_lines.push(format!("+{}", actual_lines[j]));
        j += 1;
    }

    format!("--- expected\n+++ actual\n@@\n{}", diff_lines.join("\n"))
}

fn indent_block(block: &str, prefix: &str) -> String {
    block
        .lines()
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

// ===========================================================================
// Small RSX building-blocks (pub(crate) so hydrate.rs can call them)
// ===========================================================================

pub(crate) fn missing_node_rsx() -> String {
    "missing_node {}".to_string()
}

pub(crate) fn placeholder_rsx() -> String {
    "{ VNode::placeholder() }".to_string()
}

pub(crate) fn rsx_string_literal(value: &str) -> String {
    format!("{value:?}")
}

fn rsx_name(name: &str) -> String {
    if is_simple_rsx_ident(name) {
        name.to_string()
    } else {
        rsx_string_literal(name)
    }
}

// ===========================================================================
// Classification helpers
// ===========================================================================

fn is_simple_rsx_ident(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }

    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric()) && !is_rust_keyword(name)
}

fn is_rust_keyword(name: &str) -> bool {
    matches!(
        name,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
    )
}

fn is_internal_attribute_name(name: &str) -> bool {
    name.starts_with("on") || name.starts_with("data-node") || name.starts_with("data-dioxus")
}

fn is_placeholder_comment(comment: &str) -> bool {
    comment.trim().starts_with("placeholder")
}

fn is_boolean_html_attribute(name: &str) -> bool {
    dioxus_html::BOOL_ATTRS.contains(&name)
}

fn should_skip_validation_node(node: &web_sys::Node) -> bool {
    if node.node_type() == web_sys::Node::COMMENT_NODE {
        let marker = node.text_content().unwrap_or_default();
        let marker = marker.trim();
        return marker.starts_with("node-id") || marker == "#";
    }

    let Some(element) = node.dyn_ref::<web_sys::Element>() else {
        return false;
    };

    if !element.tag_name().eq_ignore_ascii_case("script") {
        return false;
    }

    let script = node.text_content().unwrap_or_default();
    let script = script.trim();

    script.starts_with("window.hydrate_queue=")
        || script.starts_with("window.dx_hydrate(")
        || script.starts_with("window.initial_dioxus_hydration_data=")
        || script.starts_with("window.initial_dioxus_hydration_debug_types=")
        || script.starts_with("window.initial_dioxus_hydration_debug_locations=")
}

fn find_missing_attrs(
    element: &web_sys::Element,
    static_attrs: &'static [TemplateAttribute],
    dynamic_attrs: &[&[Attribute]],
) -> Vec<String> {
    let mut missing = Vec::new();

    for attr in static_attrs {
        if let TemplateAttribute::Static { name, .. } = attr {
            if name.starts_with("data-node") || name.starts_with("on") {
                continue;
            }
            if !element.has_attribute(name) {
                missing.push((*name).to_string());
            }
        }
    }

    for attrs in dynamic_attrs {
        for attr in attrs.iter() {
            if attr.name.starts_with("on") || attr.name.starts_with("data-node") {
                continue;
            }
            if matches!(attr.value, AttributeValue::None) {
                continue;
            }
            if !element.has_attribute(attr.name) {
                missing.push(attr.name.to_string());
            }
        }
    }

    missing.sort();
    missing.dedup();
    missing
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

fn describe_expected_element(tag: &str, namespace: Option<&str>) -> String {
    match namespace {
        Some(ns) => format!("<{}> (namespace: {})", tag, ns),
        None => format!("<{}>", tag),
    }
}

fn describe_missing_attrs(attrs: &[String]) -> String {
    attrs.join(", ")
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
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

        let result = validator.validate_element(None, "div", None, &[], &[], "div {}");

        assert!(!result);
        assert!(validator.has_mismatches());
        assert_eq!(validator.mismatches.len(), 1);
        assert_eq!(
            validator.mismatches[0].reason,
            "Expected <div>, found missing node."
        );
        assert_eq!(validator.mismatches[0].actual_rsx, "missing_node {}");
        assert_eq!(validator.mismatches[0].expected_rsx, "div {}");
    }

    #[test]
    fn test_validate_text_missing_node() {
        let mut validator = HydrationValidator::new();

        let result = validator.validate_text(None, "Hello", &rsx_string_literal("Hello"));

        assert!(!result);
        assert!(validator.has_mismatches());
        assert_eq!(
            validator.mismatches[0].reason,
            "Expected text \"Hello\", found missing node."
        );
        assert_eq!(validator.mismatches[0].actual_rsx, "missing_node {}");
        assert_eq!(validator.mismatches[0].expected_rsx, "\"Hello\"");
    }

    #[test]
    fn test_validate_placeholder_missing_node() {
        let mut validator = HydrationValidator::new();

        let result = validator.validate_placeholder(None, &placeholder_rsx());

        assert!(!result);
        assert!(validator.has_mismatches());
        assert_eq!(
            validator.mismatches[0].reason,
            "Expected placeholder (comment node), found missing node."
        );
        assert_eq!(validator.mismatches[0].actual_rsx, "missing_node {}");
        assert_eq!(
            validator.mismatches[0].expected_rsx,
            "{VNode::placeholder()}"
        );
    }

    #[test]
    fn test_take_mismatches() {
        let mut validator = HydrationValidator::new();
        validator.validate_element(None, "div", None, &[], &[], "div {}");

        assert!(validator.has_mismatches());

        let mismatches = validator.take_mismatches();
        assert_eq!(mismatches.len(), 1);
        assert!(!validator.has_mismatches());
    }

    #[test]
    fn test_describe_missing_attrs() {
        let attrs = vec!["role".to_string(), "title".to_string()];
        assert_eq!(super::describe_missing_attrs(&attrs), "role, title");
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

        validator.validate_element(None, "div", None, &[], &[], "div {}");

        assert_eq!(validator.mismatches[0].component_path, "App > UserProfile");
    }

    #[test]
    fn test_component_path_strips_framework_prefix() {
        let mut validator = HydrationValidator::new();
        validator.push_component("dioxus_core::suspense::component::SuspenseBoundary");
        validator.push_component("dioxus_core::error_boundary::ErrorBoundary");
        validator.push_component("root");
        validator.push_component("NestedMismatch");
        validator.push_component("NestedLeaf");

        validator.validate_element(None, "div", None, &[], &[], "div {}");

        assert_eq!(
            validator.mismatches[0].component_path,
            "NestedMismatch > NestedLeaf"
        );
    }

    #[test]
    fn test_mismatch_includes_suspense_path() {
        let mut validator = HydrationValidator::with_suspense_path(vec![1, 2, 3]);

        validator.validate_element(None, "div", None, &[], &[], "div {}");

        assert_eq!(validator.mismatches[0].suspense_path, Some(vec![1, 2, 3]));
    }

    #[test]
    fn test_unified_rsx_diff_looks_like_git_diff() {
        let expected = normalize_rsx_block(r#"strong { id: "nested-leaf", "Nested client leaf" }"#);
        let actual = normalize_rsx_block(r#"span { id: "nested-leaf", "Nested client leaf" }"#);

        let diff = unified_rsx_diff(&expected, &actual);

        assert!(diff.contains("--- expected"));
        assert!(diff.contains("+++ actual"));
        assert!(diff.contains("@@"));
        assert!(diff.contains("-strong {"));
        assert!(diff.contains("+span {"));
    }

    #[test]
    fn test_attribute_diff_contains_added_and_removed_lines() {
        let expected = normalize_rsx_block(
            r#"div { id: "attribute-mismatch", role: "status", title: "Client attribute title", "Attribute branch" }"#,
        );
        let actual = normalize_rsx_block(r#"div { id: "attribute-mismatch", "Attribute branch" }"#);

        let diff = unified_rsx_diff(&expected, &actual);

        assert!(diff.contains("role: \"status\""));
        assert!(diff.contains("title: \"Client attribute title\""));
        assert!(diff.contains("Attribute branch"));
    }

    #[test]
    fn test_autofmt_indents_multiline_children() {
        let raw = render_element_rsx(
            "section",
            vec!["id: \"placeholder-mismatch-shell\"".to_string()],
            vec![render_element_rsx(
                "p",
                vec!["id: \"server-placeholder-content\"".to_string()],
                vec![rsx_string_literal("Server placeholder content")],
            )],
        );
        let rendered = normalize_rsx_block(&raw);

        assert!(rendered.contains("\n        p {"));
        assert!(rendered.contains("\n    }"));
    }

    #[test]
    fn test_normalize_rsx_block_falls_back_to_raw_block() {
        let invalid = "div {";

        assert_eq!(normalize_rsx_block(invalid), invalid);
    }

    #[test]
    fn test_internal_hydration_markers_are_identifiable() {
        assert!(is_internal_attribute_name("data-node-hydration"));
        assert!(is_internal_attribute_name("data-dioxus-id"));
        assert!(!is_internal_attribute_name("title"));
        assert!(is_placeholder_comment("placeholder3"));
        assert!(!is_placeholder_comment("node-id3"));
    }
}
