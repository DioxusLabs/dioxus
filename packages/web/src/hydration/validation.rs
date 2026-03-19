//! Debug-mode hydration validation.
//!
//! This module is only compiled in debug builds. It validates that the
//! server-rendered DOM matches the client virtual-DOM and, on mismatch, emits a
//! human-readable RSX diff before falling back to a full client rebuild.

use dioxus_autofmt::write_block_out;
use dioxus_core::{
    Attribute, AttributeValue, DynamicNode, TemplateAttribute, TemplateNode, VNode, VirtualDom,
};
use dioxus_rsx::{
    Attribute as RsxAttribute, AttributeName as RsxAttributeName,
    AttributeValue as RsxAttributeValue, BodyNode, CallBody, Component as RsxComponent,
    Diagnostics, Element as RsxElement, ElementName as RsxElementName, ExprNode, HotLiteral,
    PartialExpr, TemplateBody, TextNode,
};
use syn::{parse::Parser, parse_quote};
use wasm_bindgen::JsCast;

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

/// Maximum number of mismatches to collect before stopping.
/// After a tag-level mismatch the traverser descends into the wrong subtree,
/// producing cascading noise. Capping the count keeps output actionable.
const MAX_MISMATCHES: usize = 5;

#[derive(Default)]
pub(crate) struct HydrationValidationSession {
    validator: HydrationValidator,
}

impl HydrationValidationSession {
    pub fn run_scope<E, F, P>(
        &mut self,
        roots: Vec<web_sys::Node>,
        suspense_path: P,
        hydrate: F,
    ) -> Result<bool, E>
    where
        F: FnOnce(&mut Self) -> Result<(), E>,
        P: FnOnce() -> Option<Vec<u32>>,
    {
        self.validator.init_traversal(roots);
        hydrate(self)?;
        let has_mismatches = self.validator.has_mismatches();
        if has_mismatches {
            let suspense_path = suspense_path();
            self.validator.report_mismatches(suspense_path.as_deref());
            self.validator.take_mismatches(suspense_path.as_deref());
        }

        Ok(has_mismatches)
    }

    pub fn element<E, F>(
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

        let current_node = self.validator.current_dom_node().cloned();
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

        self.validator.validate_element(
            current_node.as_ref(),
            tag,
            *namespace,
            attrs,
            &dynamic_attrs,
            &expected_rsx,
        );
        self.validator
            .push_element_context(expected_rsx, current_node);

        let has_children = !children.is_empty();
        if has_children {
            self.validator.push_children();
        }

        let result = hydrate(self);

        if has_children {
            self.validator.pop_children();
        }
        self.validator.pop_element_context();
        self.validator.advance();

        result
    }

    pub fn text<E, F>(&mut self, expected_content: &str, hydrate: F) -> Result<(), E>
    where
        F: FnOnce(&mut Self) -> Result<(), E>,
    {
        let current_node = self.validator.current_dom_node().cloned();
        self.validator.validate_text(
            current_node.as_ref(),
            expected_content,
            &rsx_string_literal(expected_content),
        );

        let result = hydrate(self);
        self.validator.advance();
        result
    }

    pub fn placeholder<E, F>(&mut self, hydrate: F) -> Result<(), E>
    where
        F: FnOnce(&mut Self) -> Result<(), E>,
    {
        let current_node = self.validator.current_dom_node().cloned();
        self.validator
            .validate_placeholder(current_node.as_ref(), &placeholder_rsx());

        let result = hydrate(self);
        self.validator.advance();
        result
    }

    pub fn component<E, F>(&mut self, name: &'static str, hydrate: F) -> Result<(), E>
    where
        F: FnOnce(&mut Self) -> Result<(), E>,
    {
        self.validator.push_component(name);
        let result = hydrate(self);
        self.validator.pop_component();
        result
    }
}

/// Validator that tracks component path and collects hydration mismatches
#[derive(Default)]
struct HydrationValidator {
    /// Stack of component names for path tracking
    component_stack: Vec<&'static str>,
    /// Collected mismatches
    mismatches: Vec<HydrationMismatch>,
    /// Stack of DOM traversers - one per level of recursion
    traverser_stack: Vec<DomTraverser>,
    /// Stack of expected/actual element contexts for scoping node-level diffs
    element_stack: Vec<ElementMismatchContext>,
}
impl HydrationValidator {
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

    /// Validate an element node matches expectations
    pub fn validate_element(
        &mut self,
        dom_node: Option<&web_sys::Node>,
        expected_tag: &'static str,
        expected_namespace: Option<&'static str>,
        static_attrs: &'static [TemplateAttribute],
        dynamic_attrs: &[&[Attribute]],
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
            return;
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
            return;
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
            return;
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
            return;
        }
    }

    /// Validate a text node matches expectations
    pub fn validate_text(
        &mut self,
        dom_node: Option<&web_sys::Node>,
        expected_content: &str,
        expected_rsx: &str,
    ) {
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
            return;
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
            return;
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
            return;
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
            let diff = indent_block(
                &unified_rsx_diff(&mismatch.expected_rsx, &mismatch.actual_rsx),
                "    ",
            );

            tracing::warn!(
                "[HYDRATION MISMATCH] Component: {}\n  Reason: {}\n  RSX Diff:\n{}{}\n  The subtree will be cleared and rebuilt.",
                mismatch.component_path,
                mismatch.reason,
                diff,
                suspense_info,
            );
        }
        if self.mismatches.len() >= MAX_MISMATCHES {
            tracing::warn!(
                "[HYDRATION MISMATCH] ... and potentially more. Only the first {MAX_MISMATCHES} mismatches are shown."
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
        if self.mismatches.len() >= MAX_MISMATCHES {
            return;
        }
        self.mismatches.push(HydrationMismatch {
            reason,
            expected_rsx: normalize_rsx_block(&expected_rsx),
            actual_rsx: normalize_rsx_block(&actual_rsx),
            component_path: self.component_path(),
            suspense_path: None,
        });
    }
}

pub(crate) fn serialize_template_subtree(
    dom: &VirtualDom,
    vnode: &VNode,
    node: &TemplateNode,
) -> String {
    format_rsx_nodes(serialize_template_node_items(dom, vnode, node))
}

fn serialize_template_node_items(
    dom: &VirtualDom,
    vnode: &VNode,
    node: &TemplateNode,
) -> Vec<BodyNode> {
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
            attributes.sort_by_key(attribute_sort_key);
            vec![rsx_element_node(tag, attributes, child_items)]
        }
        TemplateNode::Text { text } => vec![rsx_text_node(text)],
        TemplateNode::Dynamic { id } => serialize_dynamic_node_items(dom, vnode, *id),
    }
}

fn serialize_dynamic_node_items(
    dom: &VirtualDom,
    vnode: &VNode,
    dynamic_id: usize,
) -> Vec<BodyNode> {
    match &vnode.dynamic_nodes[dynamic_id] {
        DynamicNode::Text(text) => vec![rsx_text_node(&text.value)],
        DynamicNode::Placeholder(_) => vec![placeholder_node()],
        DynamicNode::Fragment(fragment) => fragment
            .iter()
            .flat_map(|fragment_vnode| serialize_vnode_items(dom, fragment_vnode))
            .collect(),
        DynamicNode::Component(comp) => comp
            .mounted_scope(dynamic_id, vnode, dom)
            .map(|scope| serialize_vnode_items(dom, scope.root_node()))
            .unwrap_or_else(|| vec![unmounted_component_node(comp.name)]),
    }
}

fn serialize_vnode_items(dom: &VirtualDom, vnode: &VNode) -> Vec<BodyNode> {
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
) -> Vec<RsxAttribute> {
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
                dynamic_attrs.sort_by_key(attribute_sort_key);
                rendered.extend(dynamic_attrs);
            }
        }
    }

    rendered
}

fn render_static_template_attribute(name: &str, value: &str) -> Option<RsxAttribute> {
    if is_internal_attribute_name(name) {
        return None;
    }

    let value = if is_boolean_html_attribute(name) && (value.is_empty() || value == "true") {
        let value = true;
        rsx_expr_attribute(name, parse_quote!(#value))
    } else {
        rsx_string_attribute(name, value)
    };

    Some(value)
}

fn render_dynamic_template_attribute(attr: &Attribute) -> Option<RsxAttribute> {
    if is_internal_attribute_name(attr.name)
        || matches!(
            attr.value,
            AttributeValue::Listener(_) | AttributeValue::None
        )
    {
        return None;
    }

    let rendered_value = match &attr.value {
        AttributeValue::Text(value) => rsx_string_attribute(attr.name, value),
        AttributeValue::Float(value) if value.is_finite() => {
            let value = *value;
            rsx_expr_attribute(attr.name, parse_quote!(#value))
        }
        AttributeValue::Float(_) => rsx_string_attribute(attr.name, "<non-finite-float>"),
        AttributeValue::Int(value) => {
            let value = *value;
            rsx_expr_attribute(attr.name, parse_quote!(#value))
        }
        AttributeValue::Bool(value) => {
            let value = *value;
            rsx_expr_attribute(attr.name, parse_quote!(#value))
        }
        AttributeValue::Any(_) => rsx_string_attribute(attr.name, "<any>"),
        AttributeValue::Listener(_) | AttributeValue::None => return None,
    };

    Some(rendered_value)
}

fn serialize_dom_subtree(node: &web_sys::Node) -> String {
    format_rsx_nodes(serialize_dom_node_items(node))
}

fn serialize_dom_node_items(node: &web_sys::Node) -> Vec<BodyNode> {
    if should_skip_validation_node(node) {
        return Vec::new();
    }

    match node.node_type() {
        web_sys::Node::ELEMENT_NODE => {
            let Some(element) = node.dyn_ref::<web_sys::Element>() else {
                return vec![missing_node_node()];
            };

            let mut attrs = serialize_dom_attributes(element);
            let mut children = Vec::new();
            let mut child = node.first_child();
            while let Some(current) = child {
                children.extend(serialize_dom_node_items(&current));
                child = current.next_sibling();
            }

            attrs.sort_by_key(attribute_sort_key);
            vec![rsx_element_node(
                &element.tag_name().to_lowercase(),
                attrs,
                children,
            )]
        }
        web_sys::Node::TEXT_NODE => vec![rsx_text_node(&node.text_content().unwrap_or_default())],
        web_sys::Node::COMMENT_NODE => {
            let comment = node.text_content().unwrap_or_default();
            if is_placeholder_comment(&comment) {
                vec![placeholder_node()]
            } else {
                vec![rsx_text_node(&format!("<!--{}-->", comment.trim()))]
            }
        }
        _ => vec![rsx_text_node(&format!("<node type {}>", node.node_type()))],
    }
}

fn serialize_dom_attributes(element: &web_sys::Element) -> Vec<RsxAttribute> {
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
        let attr = if is_boolean_html_attribute(&name) && (value.is_empty() || value == "true") {
            let value = true;
            rsx_expr_attribute(&name, parse_quote!(#value))
        } else {
            rsx_string_attribute(&name, &value)
        };
        rendered.push(attr);
    }

    rendered
}

fn format_rsx_nodes(nodes: Vec<BodyNode>) -> String {
    let nodes = if nodes.is_empty() {
        vec![missing_node_node()]
    } else {
        nodes
    };

    let body = CallBody::new(TemplateBody::new(nodes));
    write_block_out(&body)
        .map(normalize_formatted_rsx)
        .expect("hydration validation should always emit valid RSX")
}

fn normalize_formatted_rsx(formatted: String) -> String {
    if formatted.trim().is_empty() {
        return String::new();
    }

    let shared_indent = formatted
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(leading_whitespace)
        .reduce(shared_whitespace_prefix)
        .unwrap_or_default();

    let dedented = if shared_indent.is_empty() {
        formatted
    } else {
        formatted
            .lines()
            .map(|line| line.strip_prefix(&shared_indent).unwrap_or(line))
            .collect::<Vec<_>>()
            .join("\n")
    };

    dedented.trim().to_string()
}

fn leading_whitespace(line: &str) -> String {
    line.chars().take_while(|ch| ch.is_whitespace()).collect()
}

fn shared_whitespace_prefix(left: String, right: String) -> String {
    left.chars()
        .zip(right.chars())
        .take_while(|(left, right)| left == right)
        .map(|(ch, _)| ch)
        .collect()
}

fn rsx_element_node(
    tag: &str,
    mut attributes: Vec<RsxAttribute>,
    children: Vec<BodyNode>,
) -> BodyNode {
    attributes.sort_by_key(attribute_sort_key);
    BodyNode::Element(RsxElement {
        name: rsx_element_name(tag),
        raw_attributes: attributes.clone(),
        merged_attributes: attributes,
        spreads: Vec::new(),
        children,
        brace: Some(Default::default()),
        diagnostics: Diagnostics::new(),
    })
}

fn rsx_text_node(value: &str) -> BodyNode {
    BodyNode::Text(TextNode::from_text(value))
}

fn unmounted_component_node(name: &str) -> BodyNode {
    match syn::parse_str::<syn::Path>(name) {
        Ok(path) => BodyNode::Component(RsxComponent {
            name: path,
            generics: None,
            fields: Vec::new(),
            component_literal_dyn_idx: Vec::new(),
            spreads: Vec::new(),
            brace: Some(Default::default()),
            children: TemplateBody::new(Vec::new()),
            dyn_idx: Default::default(),
            diagnostics: Diagnostics::new(),
        }),
        Err(_) => rsx_text_node(&format!("<component {name}>")),
    }
}

fn missing_node_node() -> BodyNode {
    unmounted_component_node("missing_node")
}

fn attribute_sort_key(attr: &RsxAttribute) -> String {
    attr.name.to_string()
}

fn rsx_string_attribute(name: &str, value: &str) -> RsxAttribute {
    RsxAttribute::from_raw(
        rsx_attribute_name(name),
        RsxAttributeValue::AttrLiteral(HotLiteral::from_raw_text(value)),
    )
}

fn rsx_expr_attribute(name: &str, value: syn::Expr) -> RsxAttribute {
    RsxAttribute::from_raw(
        rsx_attribute_name(name),
        RsxAttributeValue::AttrExpr(PartialExpr::from_expr(&value)),
    )
}

fn rsx_attribute_name(name: &str) -> RsxAttributeName {
    if is_simple_rsx_ident(name) {
        RsxAttributeName::BuiltIn(syn::parse_str(name).expect("validated RSX attribute ident"))
    } else {
        RsxAttributeName::Custom(parse_quote!(#name))
    }
}

fn rsx_element_name(name: &str) -> RsxElementName {
    if is_simple_rsx_ident(name) {
        RsxElementName::Ident(syn::parse_str(name).expect("validated RSX element ident"))
    } else {
        RsxElementName::Custom(parse_quote!(#name))
    }
}

pub(crate) fn normalize_rsx_block(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return trimmed.to_string();
    }

    CallBody::parse_strict
        .parse_str(trimmed)
        .ok()
        .and_then(|body| write_block_out(&body).map(normalize_formatted_rsx))
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

pub(crate) fn missing_node_rsx() -> String {
    format_rsx_nodes(vec![missing_node_node()])
}

pub(crate) fn placeholder_rsx() -> String {
    format_rsx_nodes(vec![placeholder_node()])
}

pub(crate) fn rsx_string_literal(value: &str) -> String {
    format!("{value:?}")
}

fn placeholder_node() -> BodyNode {
    BodyNode::RawExpr(ExprNode {
        expr: syn::parse_str("{ VNode::placeholder() }")
            .expect("placeholder expression should always parse"),
        dyn_idx: Default::default(),
    })
}

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
            if is_internal_attribute_name(name) {
                continue;
            }
            if !element.has_attribute(name) {
                missing.push((*name).to_string());
            }
        }
    }

    for attrs in dynamic_attrs {
        for attr in attrs.iter() {
            if is_internal_attribute_name(attr.name) {
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

fn describe_missing_attrs(attrs: &[String]) -> String {
    attrs.join(", ")
}
