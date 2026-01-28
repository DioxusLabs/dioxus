//! Core element builder implementation.

use dioxus_core::{
    Attribute, AttributeValue, DynamicNode, HasAttributes, IntoAttributeValue, IntoDynNode,
    Template, TemplateAttribute, TemplateNode, VNode, VText,
};
use parking_lot::RwLock;
use std::collections::{HashMap, VecDeque};
use std::fmt::Arguments;

use crate::GlobalAttributesExtension;
use crate::SvgAttributesExtension;

impl GlobalAttributesExtension for ElementBuilder {}
impl SvgAttributesExtension for ElementBuilder {}

// =============================================================================
// Child Node Types (Static vs Dynamic)
// =============================================================================

/// Represents a child node that can be either static (embedded in template)
/// or dynamic (evaluated at runtime).
///
/// Static children are more performant because they are embedded directly
/// in the template and don't participate in the diffing algorithm.
#[derive(Clone)]
pub enum ChildNode {
    /// A static text node that never changes. Embedded directly in the template.
    StaticText(&'static str),
    /// A static element with static children. Embedded directly in the template.
    StaticElement(StaticElement),
    /// A dynamic node that may change. Requires runtime diffing.
    Dynamic(DynamicNode),
}

/// A static element that can be embedded in the template.
#[derive(Clone)]
pub struct StaticElement {
    pub tag: &'static str,
    pub namespace: Option<&'static str>,
    pub attrs: &'static [StaticAttribute],
    pub children: Vec<ChildNode>,
}

/// A static attribute embedded in the template.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct StaticAttribute {
    pub name: &'static str,
    pub value: &'static str,
    pub namespace: Option<&'static str>,
}

// =============================================================================
// Debug Implementations
// =============================================================================

impl std::fmt::Debug for ChildNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChildNode::StaticText(s) => write!(f, "StaticText({:?})", s),
            ChildNode::StaticElement(e) => write!(f, "StaticElement(<{}>)", e.tag),
            ChildNode::Dynamic(_) => write!(f, "Dynamic(...)"),
        }
    }
}

impl std::fmt::Debug for StaticElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StaticElement")
            .field("tag", &self.tag)
            .field("namespace", &self.namespace)
            .field("attrs", &self.attrs)
            .field("children_count", &self.children.len())
            .finish()
    }
}

impl std::fmt::Debug for StaticAttribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}={:?}", self.name, self.value)
    }
}

// =============================================================================
// Template Cache
// =============================================================================

const TEMPLATE_CACHE_CAP: usize = 1024;

/// A hashable key for static attributes.
#[derive(Clone, PartialEq, Eq, Hash)]
struct StaticAttributeKey {
    name: &'static str,
    value: &'static str,
    namespace: Option<&'static str>,
}

impl From<&StaticAttribute> for StaticAttributeKey {
    fn from(attr: &StaticAttribute) -> Self {
        Self {
            name: attr.name,
            value: attr.value,
            namespace: attr.namespace,
        }
    }
}

/// A hashable key representing a static element for cache lookup.
/// This is recursive to handle nested static elements.
#[derive(Clone, PartialEq, Eq, Hash)]
struct StaticElementKey {
    tag: &'static str,
    namespace: Option<&'static str>,
    attrs: Vec<StaticAttributeKey>,
    children: Vec<ChildPattern>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum ChildPattern {
    StaticText(&'static str),
    StaticElement(StaticElementKey),
    Dynamic,
}

fn static_element_to_key(elem: &StaticElement) -> StaticElementKey {
    StaticElementKey {
        tag: elem.tag,
        namespace: elem.namespace,
        attrs: elem.attrs.iter().map(StaticAttributeKey::from).collect(),
        children: elem.children.iter().map(child_to_pattern).collect(),
    }
}

fn child_to_pattern(child: &ChildNode) -> ChildPattern {
    match child {
        ChildNode::StaticText(s) => ChildPattern::StaticText(s),
        ChildNode::StaticElement(e) => ChildPattern::StaticElement(static_element_to_key(e)),
        ChildNode::Dynamic(_) => ChildPattern::Dynamic,
    }
}

/// Key for caching templates with mixed static/dynamic children.
#[derive(Clone, PartialEq, Eq, Hash)]
struct HybridTemplateKey {
    tag: &'static str,
    namespace: Option<&'static str>,
    child_pattern: Vec<ChildPattern>,
    has_attributes: bool,
}

/// Cache for hybrid templates (with mixed static/dynamic children)
struct HybridTemplateCache {
    map: HashMap<HybridTemplateKey, Template>,
    order: VecDeque<HybridTemplateKey>,
}

impl HybridTemplateCache {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    fn get(&self, key: &HybridTemplateKey) -> Option<Template> {
        self.map.get(key).copied()
    }

    fn insert(&mut self, key: HybridTemplateKey, template: Template) {
        if self.map.contains_key(&key) {
            return;
        }

        self.map.insert(key.clone(), template);
        self.order.push_back(key);

        if self.order.len() > TEMPLATE_CACHE_CAP {
            if let Some(oldest) = self.order.pop_front() {
                self.map.remove(&oldest);
            }
        }
    }
}

static HYBRID_TEMPLATES: RwLock<Option<HybridTemplateCache>> = RwLock::new(None);

const DYNAMIC_ROOT_PATH: [u8; 1] = [0];
const DYNAMIC_ROOT_PATHS: [&[u8]; 1] = [&DYNAMIC_ROOT_PATH];
const DYNAMIC_ROOTS: [TemplateNode; 1] = [TemplateNode::Dynamic { id: 0 }];
const DYNAMIC_ROOT_TEMPLATE: Template = Template {
    roots: &DYNAMIC_ROOTS,
    node_paths: &DYNAMIC_ROOT_PATHS,
    attr_paths: &[],
};

/// Get or create a hybrid template with mixed static/dynamic children.
fn get_hybrid_template(
    tag: &'static str,
    namespace: Option<&'static str>,
    children: &[ChildNode],
    has_attributes: bool,
) -> Template {
    let child_pattern: Vec<ChildPattern> = children.iter().map(child_to_pattern).collect();

    let key = HybridTemplateKey {
        tag,
        namespace,
        child_pattern,
        has_attributes,
    };

    if let Some(template) = HYBRID_TEMPLATES
        .read()
        .as_ref()
        .and_then(|cache| cache.get(&key))
    {
        return template;
    }

    let mut write = HYBRID_TEMPLATES.write();
    let cache = write.get_or_insert_with(HybridTemplateCache::new);
    if let Some(template) = cache.get(&key) {
        return template;
    }

    let template = create_hybrid_template(tag, namespace, children, has_attributes);
    cache.insert(key, template);
    template
}

/// Create a hybrid template with mixed static/dynamic children.
fn create_hybrid_template(
    tag: &'static str,
    namespace: Option<&'static str>,
    children: &[ChildNode],
    has_attributes: bool,
) -> Template {
    let mut template_children = Vec::with_capacity(children.len());
    let mut node_paths = Vec::new();
    let mut dynamic_id = 0usize;

    for (i, child) in children.iter().enumerate() {
        match child {
            ChildNode::StaticText(text) => {
                // Static text is embedded directly in the template
                template_children.push(TemplateNode::Text { text });
            }
            ChildNode::StaticElement(elem) => {
                // Static element is embedded in the template
                template_children.push(child_node_to_template_node(elem));
            }
            ChildNode::Dynamic(_) => {
                // Dynamic node gets a placeholder in the template
                template_children.push(TemplateNode::Dynamic { id: dynamic_id });
                let path: &'static [u8] = Box::leak(Box::new([0u8, i as u8]));
                node_paths.push(path);
                dynamic_id += 1;
            }
        }
    }

    let template_children: &'static [TemplateNode] =
        Box::leak(template_children.into_boxed_slice());
    let node_paths: &'static [&'static [u8]] = Box::leak(node_paths.into_boxed_slice());

    let mut attrs_list = Vec::with_capacity(1);
    if has_attributes {
        attrs_list.push(TemplateAttribute::Dynamic { id: 0 });
    }
    let attrs: &'static [TemplateAttribute] = Box::leak(attrs_list.into_boxed_slice());

    let roots: &'static [TemplateNode] = Box::leak(Box::new([TemplateNode::Element {
        tag,
        namespace,
        attrs,
        children: template_children,
    }]));

    let attr_paths: &'static [&'static [u8]] = if has_attributes {
        Box::leak(Box::new([Box::leak(Box::new([0u8])) as &'static [u8]]))
    } else {
        &[]
    };

    Template {
        roots,
        node_paths,
        attr_paths,
    }
}

/// Convert a StaticElement to a TemplateNode (recursive).
fn child_node_to_template_node(elem: &StaticElement) -> TemplateNode {
    let children: Vec<TemplateNode> = elem
        .children
        .iter()
        .map(|child| match child {
            ChildNode::StaticText(text) => TemplateNode::Text { text },
            ChildNode::StaticElement(e) => child_node_to_template_node(e),
            ChildNode::Dynamic(_) => {
                // This shouldn't happen in a fully static element
                // but we handle it gracefully
                panic!("StaticElement cannot contain dynamic children")
            }
        })
        .collect();

    let children: &'static [TemplateNode] = Box::leak(children.into_boxed_slice());

    let attrs: Vec<TemplateAttribute> = elem
        .attrs
        .iter()
        .map(|a| TemplateAttribute::Static {
            name: a.name,
            value: a.value,
            namespace: a.namespace,
        })
        .collect();
    let attrs: &'static [TemplateAttribute] = Box::leak(attrs.into_boxed_slice());

    TemplateNode::Element {
        tag: elem.tag,
        namespace: elem.namespace,
        attrs,
        children,
    }
}

/// A builder for constructing HTML elements with a fluent API.
///
/// Supports both static and dynamic children for optimal performance.
/// Use `.static_text()` for text that never changes (embedded in template),
/// and `.child()` for dynamic content that may change at runtime.
///
/// # Example
///
/// ```rust,ignore
/// div()
///     .class("my-class")
///     .id("my-id")
///     .static_text("Label: ")        // Static - embedded in template
///     .child(dynamic_value)           // Dynamic - diffed at runtime
///     .onclick(|_| {})
///     .build()
/// ```
#[derive(Default)]
pub struct ElementBuilder {
    tag: &'static str,
    namespace: Option<&'static str>,
    attributes: Vec<Attribute>,
    children: Vec<ChildNode>,
    key: Option<String>,
}

impl std::fmt::Debug for ElementBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ElementBuilder")
            .field("tag", &self.tag)
            .field("namespace", &self.namespace)
            .field("attributes_count", &self.attributes.len())
            .field("children_count", &self.children.len())
            .field("key", &self.key)
            .finish()
    }
}

impl ElementBuilder {
    /// Create a new ElementBuilder for the given tag.
    pub fn new(tag: &'static str) -> Self {
        Self {
            tag,
            namespace: None,
            attributes: Vec::new(),
            children: Vec::new(),
            key: None,
        }
    }

    /// Create a new ElementBuilder with a namespace (e.g., for SVG elements).
    pub fn new_with_namespace(tag: &'static str, namespace: &'static str) -> Self {
        Self {
            tag,
            namespace: Some(namespace),
            attributes: Vec::new(),
            children: Vec::new(),
            key: None,
        }
    }

    /// Set the key for this element (used for list reconciliation).
    pub fn key(mut self, key: impl ToString) -> Self {
        self.key = Some(key.to_string());
        self
    }

    /// Add a dynamic child element or text node.
    ///
    /// Dynamic children are evaluated at runtime and participate in diffing.
    /// For static text that never changes, use `.static_text()` instead.
    pub fn child(mut self, child: impl IntoDynNode) -> Self {
        self.children
            .push(ChildNode::Dynamic(child.into_dyn_node()));
        self
    }

    /// Add a static text child that never changes.
    ///
    /// Static text is embedded directly in the template and does NOT participate
    /// in diffing, making it more performant than dynamic text.
    ///
    /// **Important**: The text must be a `&'static str` (compile-time string literal).
    ///
    /// For **guaranteed** const evaluation, use the [`static_str!`] macro:
    /// ```rust,ignore
    /// use dioxus::prelude::*;
    ///
    /// div()
    ///     .pipe(static_str!("Hello, "))  // Guaranteed const, embedded in template
    ///     .child(user_name)               // Dynamic, will be diffed
    ///     .pipe(static_str!("!"))         // Guaranteed const, embedded in template
    ///     .build()
    ///
    /// // Or using the two-argument form:
    /// let builder = div();
    /// static_str!(builder, "Hello!")
    ///     .build()
    /// ```
    ///
    /// Or use the method directly with string literals:
    /// ```rust,ignore
    /// div()
    ///     .static_text("Hello, ")     // Embedded in template
    ///     .child(user_name)            // Dynamic, will be diffed
    ///     .static_text("!")            // Embedded in template
    ///     .build()
    /// ```
    pub fn static_text(mut self, text: &'static str) -> Self {
        self.children.push(ChildNode::StaticText(text));
        self
    }

    /// Add a static element child that never changes.
    ///
    /// Static elements are embedded directly in the template and do NOT
    /// participate in diffing, making them more performant.
    ///
    /// # Example
    /// ```rust,ignore
    /// div()
    ///     .static_element(StaticElement {
    ///         tag: "span",
    ///         namespace: None,
    ///         attrs: &[StaticAttribute { name: "class", value: "icon", namespace: None }],
    ///         children: vec![ChildNode::StaticText("★")],
    ///     })
    ///     .child(dynamic_content)
    ///     .build()
    /// ```
    pub fn static_element(mut self, element: StaticElement) -> Self {
        self.children.push(ChildNode::StaticElement(element));
        self
    }

    /// Add a child element or text node only if the condition is true.
    pub fn child_if(self, condition: bool, child: impl IntoDynNode) -> Self {
        if condition {
            self.child(child)
        } else {
            self
        }
    }

    /// Add a child element or text node from one of two branches.
    pub fn child_if_else(
        self,
        condition: bool,
        then_child: impl IntoDynNode,
        else_child: impl IntoDynNode,
    ) -> Self {
        if condition {
            self.child(then_child)
        } else {
            self.child(else_child)
        }
    }

    /// Add multiple dynamic children from an iterator.
    pub fn children(mut self, children: impl IntoIterator<Item = impl IntoDynNode>) -> Self {
        for child in children {
            self.children
                .push(ChildNode::Dynamic(child.into_dyn_node()));
        }
        self
    }

    /// Add multiple keyed children from an iterator.
    ///
    /// This is a convenience method for adding children with keys for efficient
    /// list reconciliation. Each item is passed to both a key function and a
    /// child builder function.
    ///
    /// # Example
    /// ```rust,ignore
    /// ul().children_keyed(
    ///     items,
    ///     |item| item.id.to_string(),
    ///     |item| li().child(item.name)
    /// ).build()
    /// ```
    pub fn children_keyed<I, T, K, F>(mut self, items: I, key_fn: K, child_fn: F) -> Self
    where
        I: IntoIterator<Item = T>,
        K: Fn(&T) -> String,
        F: Fn(T) -> ElementBuilder,
    {
        for item in items {
            let key = key_fn(&item);
            self.children
                .push(ChildNode::Dynamic(child_fn(item).key(key).into_dyn_node()));
        }
        self
    }

    /// Convenience method for adding dynamic text content.
    ///
    /// This is equivalent to `.child(value.to_string())`.
    /// For static text, use `.static_text()` instead.
    pub fn text(self, value: impl ToString) -> Self {
        self.child(value.to_string())
    }

    /// Add a child only if the Option is Some.
    pub fn child_option(self, child: Option<impl IntoDynNode>) -> Self {
        if let Some(c) = child {
            self.child(c)
        } else {
            self
        }
    }

    /// Build the element into a VNode (Element).
    pub fn build(self) -> dioxus_core::Element {
        let has_attributes = !self.attributes.is_empty();
        self.build_hybrid(has_attributes)
    }

    /// Build with hybrid template (mixed static/dynamic children).
    fn build_hybrid(self, has_attributes: bool) -> dioxus_core::Element {
        let template =
            get_hybrid_template(self.tag, self.namespace, &self.children, has_attributes);

        // Only extract dynamic nodes
        let dynamic_nodes: Vec<DynamicNode> = self
            .children
            .into_iter()
            .filter_map(|c| match c {
                ChildNode::Dynamic(d) => Some(d),
                _ => None, // Static children are embedded in template
            })
            .collect();
        let dynamic_nodes = dynamic_nodes.into_boxed_slice();

        // Pack all attributes into a single dynamic attribute group
        let mut dynamic_attrs = Vec::new();
        if has_attributes {
            let mut attributes = self.attributes;
            merge_class_and_style_attributes(&mut attributes);
            attributes.sort_by(|a, b| a.name.cmp(b.name));
            dynamic_attrs.push(attributes.into_boxed_slice());
        }
        let dynamic_attrs = dynamic_attrs.into_boxed_slice();

        Ok(VNode::new(self.key, template, dynamic_nodes, dynamic_attrs))
    }
}

// =============================================================================
// Fragment Builder
// =============================================================================

fn text_vnode(value: impl ToString) -> VNode {
    VNode::new(
        None,
        DYNAMIC_ROOT_TEMPLATE,
        Box::new([DynamicNode::Text(VText::new(value))]),
        Box::new([]),
    )
}

/// Create an element containing a single text node.
pub fn text_node(value: impl ToString) -> dioxus_core::Element {
    Ok(text_vnode(value))
}

/// A builder for constructing fragments with multiple root nodes.
#[derive(Default)]
pub struct FragmentBuilder {
    children: Vec<VNode>,
    key: Option<String>,
}

impl std::fmt::Debug for FragmentBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FragmentBuilder")
            .field("children_count", &self.children.len())
            .field("key", &self.key)
            .finish()
    }
}

impl FragmentBuilder {
    /// Create a new FragmentBuilder.
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            key: None,
        }
    }

    /// Set the key for this fragment (used for list reconciliation).
    pub fn key(mut self, key: impl ToString) -> Self {
        self.key = Some(key.to_string());
        self
    }

    /// Add a child node.
    pub fn child(mut self, child: impl IntoFragmentChild) -> Self {
        self.children.push(child.into_fragment_child());
        self
    }

    /// Add a child node only if the condition is true.
    pub fn child_if(self, condition: bool, child: impl IntoFragmentChild) -> Self {
        if condition {
            self.child(child)
        } else {
            self
        }
    }

    /// Add a child node from one of two branches.
    pub fn child_if_else(
        self,
        condition: bool,
        then_child: impl IntoFragmentChild,
        else_child: impl IntoFragmentChild,
    ) -> Self {
        if condition {
            self.child(then_child)
        } else {
            self.child(else_child)
        }
    }

    /// Add multiple children from an iterator.
    pub fn children<I, C>(mut self, children: I) -> Self
    where
        I: IntoIterator<Item = C>,
        C: IntoFragmentChild,
    {
        for child in children {
            self.children.push(child.into_fragment_child());
        }
        self
    }

    /// Build the fragment into a VNode (Element).
    pub fn build(self) -> dioxus_core::Element {
        if self.children.is_empty() {
            VNode::empty()
        } else {
            Ok(VNode::new(
                self.key,
                DYNAMIC_ROOT_TEMPLATE,
                Box::new([DynamicNode::Fragment(self.children)]),
                Box::new([]),
            ))
        }
    }
}

/// Create a new fragment builder.
pub fn fragment() -> FragmentBuilder {
    FragmentBuilder::new()
}

/// Convert values into fragment children.
pub trait IntoFragmentChild {
    /// Consume this item and produce a VNode suitable for fragment children.
    fn into_fragment_child(self) -> VNode;
}

impl IntoFragmentChild for VNode {
    fn into_fragment_child(self) -> VNode {
        self
    }
}

impl IntoFragmentChild for &VNode {
    fn into_fragment_child(self) -> VNode {
        self.clone()
    }
}

impl IntoFragmentChild for dioxus_core::Element {
    fn into_fragment_child(self) -> VNode {
        self.unwrap_or_default()
    }
}

impl IntoFragmentChild for &dioxus_core::Element {
    fn into_fragment_child(self) -> VNode {
        self.as_ref().cloned().unwrap_or_default()
    }
}

impl IntoFragmentChild for Option<VNode> {
    fn into_fragment_child(self) -> VNode {
        self.unwrap_or_default()
    }
}

impl IntoFragmentChild for Option<dioxus_core::Element> {
    fn into_fragment_child(self) -> VNode {
        match self {
            Some(Ok(vnode)) => vnode,
            _ => VNode::default(),
        }
    }
}

impl IntoFragmentChild for ElementBuilder {
    fn into_fragment_child(self) -> VNode {
        self.build().unwrap_or_default()
    }
}

impl IntoFragmentChild for &str {
    fn into_fragment_child(self) -> VNode {
        text_vnode(self)
    }
}

impl IntoFragmentChild for String {
    fn into_fragment_child(self) -> VNode {
        text_vnode(self)
    }
}

impl IntoFragmentChild for Arguments<'_> {
    fn into_fragment_child(self) -> VNode {
        text_vnode(self)
    }
}

fn merge_class_and_style_attributes(attributes: &mut Vec<Attribute>) {
    if attributes.len() < 2 {
        return;
    }

    let mut merged_classes: Vec<String> = Vec::new();
    let mut merged_styles: Vec<String> = Vec::new();
    let mut class_volatile = false;
    let mut style_volatile = false;
    let mut retained: Vec<Attribute> = Vec::with_capacity(attributes.len());

    for attr in attributes.drain(..) {
        if attr.name == "class" && attr.namespace.is_none() {
            class_volatile |= attr.volatile;
            match attr.value {
                AttributeValue::Text(value) => {
                    if !value.is_empty() {
                        merged_classes.push(value);
                    }
                }
                AttributeValue::Int(value) => merged_classes.push(value.to_string()),
                AttributeValue::Float(value) => merged_classes.push(value.to_string()),
                AttributeValue::Bool(value) => merged_classes.push(value.to_string()),
                AttributeValue::None => {}
                other => {
                    retained.push(Attribute {
                        value: other,
                        ..attr
                    });
                }
            }
        } else if attr.name == "style" && attr.namespace.is_none() {
            style_volatile |= attr.volatile;
            match attr.value {
                AttributeValue::Text(value) => {
                    if !value.is_empty() {
                        merged_styles.push(value);
                    }
                }
                AttributeValue::None => {}
                other => {
                    retained.push(Attribute {
                        value: other,
                        ..attr
                    });
                }
            }
        } else {
            retained.push(attr);
        }
    }

    if !merged_classes.is_empty() {
        retained.push(Attribute {
            name: "class",
            namespace: None,
            value: AttributeValue::Text(merged_classes.join(" ")),
            volatile: class_volatile,
        });
    }

    if !merged_styles.is_empty() {
        retained.push(Attribute {
            name: "style",
            namespace: None,
            value: AttributeValue::Text(merged_styles.join("; ")),
            volatile: style_volatile,
        });
    }

    *attributes = retained;
}

impl IntoDynNode for ElementBuilder {
    fn into_dyn_node(self) -> DynamicNode {
        self.build()
            .map(|v| DynamicNode::Fragment(vec![v]))
            .unwrap_or_default()
    }
}

impl HasAttributes for ElementBuilder {
    fn push_attribute<T>(
        mut self,
        name: &'static str,
        ns: Option<&'static str>,
        attr: impl IntoAttributeValue<T>,
        volatile: bool,
    ) -> Self {
        self.attributes.push(Attribute {
            name,
            namespace: ns,
            value: attr.into_value(),
            volatile,
        });
        self
    }
}

// =============================================================================
// Attribute Helpers
// =============================================================================

impl ElementBuilder {
    /// Set a custom attribute with a static name.
    pub fn attr<T>(self, name: &'static str, value: impl IntoAttributeValue<T>) -> Self {
        self.push_attribute(name, None, value, false)
    }

    /// Set a custom attribute with a static name and namespace.
    pub fn attr_ns<T>(
        self,
        name: &'static str,
        namespace: &'static str,
        value: impl IntoAttributeValue<T>,
    ) -> Self {
        self.push_attribute(name, Some(namespace), value, false)
    }

    /// Set a custom attribute only when the condition is true.
    pub fn attr_if<T>(
        self,
        condition: bool,
        name: &'static str,
        value: impl IntoAttributeValue<T>,
    ) -> Self {
        if condition {
            self.push_attribute(name, None, value, false)
        } else {
            self
        }
    }

    /// Append a list of pre-built attributes.
    pub fn attrs(self, attrs: impl IntoIterator<Item = Attribute>) -> Self {
        attrs.into_iter().fold(self, |builder, attr| {
            builder.push_attribute(attr.name, attr.namespace, attr.value, attr.volatile)
        })
    }

    /// Conditionally add a class name.
    pub fn class_if(self, condition: bool, value: impl IntoAttributeValue) -> Self {
        if condition {
            self.class(value)
        } else {
            self
        }
    }

    /// Add multiple class names from an iterator.
    pub fn class_list<I, S>(self, classes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let joined = classes
            .into_iter()
            .map(|c| c.as_ref().to_string())
            .filter(|c| !c.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if joined.is_empty() {
            self
        } else {
            self.class(joined)
        }
    }

    /// Apply a function to this builder, enabling composable style helpers.
    ///
    /// This is an alias for `pipe` but with a clearer name for the common use case
    /// of applying reusable style transformations.
    ///
    /// # Example
    /// ```rust,ignore
    /// fn card_styles(b: ElementBuilder) -> ElementBuilder {
    ///     b.class("p-4 rounded shadow")
    /// }
    ///
    /// fn hover_effect(b: ElementBuilder) -> ElementBuilder {
    ///     b.class("hover:shadow-lg transition-shadow")
    /// }
    ///
    /// div()
    ///     .with(card_styles)
    ///     .with(hover_effect)
    ///     .child("Content")
    ///     .build()
    /// ```
    pub fn with<F>(self, f: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        f(self)
    }
}

// =============================================================================
// Global Attributes (available on all HTML elements)
// =============================================================================

impl ElementBuilder {
    /// Set the class attribute.
    pub fn class(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("class", None, value, false)
    }

    /// Set the id attribute.
    pub fn id(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("id", None, value, false)
    }

    /// Set the style attribute.
    pub fn style(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("style", None, value, false)
    }

    /// Add a single CSS style property.
    ///
    /// Multiple calls are automatically merged into a single style attribute.
    ///
    /// # Example
    /// ```rust,ignore
    /// div()
    ///     .style_prop("display", "flex")
    ///     .style_prop("gap", "1rem")
    ///     .build()
    /// // Results in: style="display: flex; gap: 1rem"
    /// ```
    pub fn style_prop(self, property: &'static str, value: impl ToString) -> Self {
        let style_value = format!("{}: {}", property, value.to_string());
        self.push_attribute("style", None, style_value, false)
    }

    /// Set the title attribute.
    pub fn title(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("title", None, value, false)
    }

    /// Set the hidden attribute.
    pub fn hidden(self, value: bool) -> Self {
        self.push_attribute("hidden", None, value, false)
    }

    /// Set the tabindex attribute.
    pub fn tabindex(self, value: i32) -> Self {
        self.push_attribute("tabindex", None, value, false)
    }

    /// Set the role attribute (ARIA).
    pub fn role(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("role", None, value, false)
    }

    /// Set the draggable attribute.
    pub fn draggable(self, value: bool) -> Self {
        self.push_attribute("draggable", None, value.to_string(), false)
    }

    /// Set a data-* attribute.
    pub fn data(self, name: &'static str, value: impl IntoAttributeValue) -> Self {
        // Note: For now, we use a static name. A more dynamic approach would need changes.
        self.push_attribute(name, None, value, false)
    }
}

// =============================================================================
// Event Handlers (generated from dioxus-html)
// =============================================================================

include!(concat!(env!("OUT_DIR"), "/builder_events.rs"));

// =============================================================================
// Form Element Attributes
// =============================================================================

impl ElementBuilder {
    /// Set the disabled attribute.
    pub fn disabled(self, value: bool) -> Self {
        self.push_attribute("disabled", None, value, false)
    }

    /// Set the value attribute.
    pub fn value(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("value", None, value, true) // volatile for controlled inputs
    }

    /// Set the placeholder attribute.
    pub fn placeholder(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("placeholder", None, value, false)
    }

    /// Set the name attribute.
    pub fn name(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("name", None, value, false)
    }

    /// Set the type attribute (for input elements).
    pub fn r#type(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("type", None, value, false)
    }

    /// Set the checked attribute (for checkboxes/radios).
    pub fn checked(self, value: bool) -> Self {
        self.push_attribute("checked", None, value, true) // volatile for controlled inputs
    }

    /// Set the readonly attribute.
    pub fn readonly(self, value: bool) -> Self {
        self.push_attribute("readonly", None, value, false)
    }

    /// Set the required attribute.
    pub fn required(self, value: bool) -> Self {
        self.push_attribute("required", None, value, false)
    }

    /// Set the maxlength attribute.
    pub fn maxlength(self, value: i32) -> Self {
        self.push_attribute("maxlength", None, value, false)
    }

    /// Set the minlength attribute.
    pub fn minlength(self, value: i32) -> Self {
        self.push_attribute("minlength", None, value, false)
    }

    /// Set the min attribute.
    pub fn min(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("min", None, value, false)
    }

    /// Set the max attribute.
    pub fn max(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("max", None, value, false)
    }

    /// Set the step attribute.
    pub fn step(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("step", None, value, false)
    }

    /// Set the autocomplete attribute.
    pub fn autocomplete(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("autocomplete", None, value, false)
    }
}

// =============================================================================
// Link/Anchor Attributes
// =============================================================================

impl ElementBuilder {
    /// Set the href attribute.
    pub fn href(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("href", None, value, false)
    }

    /// Set the target attribute.
    pub fn target(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("target", None, value, false)
    }

    /// Set the rel attribute.
    pub fn rel(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("rel", None, value, false)
    }

    /// Set the download attribute.
    pub fn download(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("download", None, value, false)
    }
}

// =============================================================================
// Image/Media Attributes
// =============================================================================

impl ElementBuilder {
    /// Set the src attribute.
    pub fn src(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("src", None, value, false)
    }

    /// Set the alt attribute.
    pub fn alt(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("alt", None, value, false)
    }

    /// Set the width attribute.
    pub fn width(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("width", None, value, false)
    }

    /// Set the height attribute.
    pub fn height(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("height", None, value, false)
    }

    /// Set the loading attribute (lazy/eager).
    pub fn loading(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("loading", None, value, false)
    }
}

// =============================================================================
// Table Attributes
// =============================================================================

impl ElementBuilder {
    /// Set the colspan attribute.
    pub fn colspan(self, value: i32) -> Self {
        self.push_attribute("colspan", None, value, false)
    }

    /// Set the rowspan attribute.
    pub fn rowspan(self, value: i32) -> Self {
        self.push_attribute("rowspan", None, value, false)
    }
}

// =============================================================================
// ARIA Attributes
// =============================================================================

impl ElementBuilder {
    /// Set the aria-label attribute.
    pub fn aria_label(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("aria-label", None, value, false)
    }

    /// Set the aria-labelledby attribute.
    pub fn aria_labelledby(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("aria-labelledby", None, value, false)
    }

    /// Set the aria-describedby attribute.
    pub fn aria_describedby(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("aria-describedby", None, value, false)
    }

    /// Set the aria-hidden attribute.
    pub fn aria_hidden(self, value: bool) -> Self {
        self.push_attribute("aria-hidden", None, value.to_string(), false)
    }

    /// Set the aria-expanded attribute.
    pub fn aria_expanded(self, value: bool) -> Self {
        self.push_attribute("aria-expanded", None, value.to_string(), false)
    }

    /// Set the aria-pressed attribute.
    pub fn aria_pressed(self, value: bool) -> Self {
        self.push_attribute("aria-pressed", None, value.to_string(), false)
    }

    /// Set the aria-selected attribute.
    pub fn aria_selected(self, value: bool) -> Self {
        self.push_attribute("aria-selected", None, value.to_string(), false)
    }

    /// Set the aria-checked attribute.
    pub fn aria_checked(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("aria-checked", None, value, false)
    }

    /// Set the aria-disabled attribute.
    pub fn aria_disabled(self, value: bool) -> Self {
        self.push_attribute("aria-disabled", None, value.to_string(), false)
    }

    /// Set the aria-invalid attribute.
    pub fn aria_invalid(self, value: bool) -> Self {
        self.push_attribute("aria-invalid", None, value.to_string(), false)
    }

    /// Set the aria-live attribute (off, polite, assertive).
    pub fn aria_live(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("aria-live", None, value, false)
    }

    /// Set the aria-atomic attribute.
    pub fn aria_atomic(self, value: bool) -> Self {
        self.push_attribute("aria-atomic", None, value.to_string(), false)
    }

    /// Set the aria-busy attribute.
    pub fn aria_busy(self, value: bool) -> Self {
        self.push_attribute("aria-busy", None, value.to_string(), false)
    }

    /// Set the aria-current attribute.
    pub fn aria_current(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("aria-current", None, value, false)
    }

    /// Set the aria-haspopup attribute.
    pub fn aria_haspopup(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("aria-haspopup", None, value, false)
    }

    /// Set the aria-controls attribute.
    pub fn aria_controls(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("aria-controls", None, value, false)
    }

    /// Set the aria-owns attribute.
    pub fn aria_owns(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("aria-owns", None, value, false)
    }

    /// Set the aria-valuemin attribute.
    pub fn aria_valuemin(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("aria-valuemin", None, value, false)
    }

    /// Set the aria-valuemax attribute.
    pub fn aria_valuemax(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("aria-valuemax", None, value, false)
    }

    /// Set the aria-valuenow attribute.
    pub fn aria_valuenow(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("aria-valuenow", None, value, false)
    }

    /// Set the aria-valuetext attribute.
    pub fn aria_valuetext(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("aria-valuetext", None, value, false)
    }

    /// Set the aria-modal attribute.
    pub fn aria_modal(self, value: bool) -> Self {
        self.push_attribute("aria-modal", None, value.to_string(), false)
    }
}

// =============================================================================
// SVG Attributes
// =============================================================================

impl ElementBuilder {
    /// Set the viewBox attribute (SVG).
    #[allow(non_snake_case)]
    pub fn viewBox(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("viewBox", None, value, false)
    }

    /// Set the fill attribute (SVG).
    pub fn fill(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("fill", None, value, false)
    }

    /// Set the stroke attribute (SVG).
    pub fn stroke(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("stroke", None, value, false)
    }

    /// Set the stroke-width attribute (SVG).
    pub fn stroke_width(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("stroke-width", None, value, false)
    }

    /// Set the stroke-linecap attribute (SVG).
    pub fn stroke_linecap(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("stroke-linecap", None, value, false)
    }

    /// Set the stroke-linejoin attribute (SVG).
    pub fn stroke_linejoin(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("stroke-linejoin", None, value, false)
    }

    /// Set the stroke-dasharray attribute (SVG).
    pub fn stroke_dasharray(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("stroke-dasharray", None, value, false)
    }

    /// Set the stroke-dashoffset attribute (SVG).
    pub fn stroke_dashoffset(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("stroke-dashoffset", None, value, false)
    }

    /// Set the d attribute (SVG path data).
    pub fn d(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("d", None, value, false)
    }

    /// Set the cx attribute (SVG circle/ellipse center x).
    pub fn cx(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("cx", None, value, false)
    }

    /// Set the cy attribute (SVG circle/ellipse center y).
    pub fn cy(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("cy", None, value, false)
    }

    /// Set the r attribute (SVG circle radius).
    pub fn r(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("r", None, value, false)
    }

    /// Set the rx attribute (SVG ellipse/rect x radius).
    pub fn rx(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("rx", None, value, false)
    }

    /// Set the ry attribute (SVG ellipse/rect y radius).
    pub fn ry(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("ry", None, value, false)
    }

    /// Set the x attribute (SVG).
    pub fn x(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("x", None, value, false)
    }

    /// Set the y attribute (SVG).
    pub fn y(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("y", None, value, false)
    }

    /// Set the x1 attribute (SVG line).
    pub fn x1(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("x1", None, value, false)
    }

    /// Set the y1 attribute (SVG line).
    pub fn y1(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("y1", None, value, false)
    }

    /// Set the x2 attribute (SVG line).
    pub fn x2(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("x2", None, value, false)
    }

    /// Set the y2 attribute (SVG line).
    pub fn y2(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("y2", None, value, false)
    }

    /// Set the points attribute (SVG polygon/polyline).
    pub fn points(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("points", None, value, false)
    }

    /// Set the transform attribute (SVG).
    pub fn transform(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("transform", None, value, false)
    }

    /// Set the opacity attribute (SVG).
    pub fn opacity(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("opacity", None, value, false)
    }

    /// Set the fill-opacity attribute (SVG).
    pub fn fill_opacity(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("fill-opacity", None, value, false)
    }

    /// Set the stroke-opacity attribute (SVG).
    pub fn stroke_opacity(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("stroke-opacity", None, value, false)
    }

    /// Set the clip-path attribute (SVG).
    pub fn clip_path(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("clip-path", None, value, false)
    }

    /// Set the preserveAspectRatio attribute (SVG).
    #[allow(non_snake_case)]
    pub fn preserveAspectRatio(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("preserveAspectRatio", None, value, false)
    }

    /// Set the fill-rule attribute (SVG).
    pub fn fill_rule(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("fill-rule", None, value, false)
    }

    /// Set the clip-rule attribute (SVG).
    pub fn clip_rule(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("clip-rule", None, value, false)
    }
}

// =============================================================================
// Element Constructor Functions
// =============================================================================

// Document Metadata
pub fn head() -> ElementBuilder {
    ElementBuilder::new("head")
}
pub fn title() -> ElementBuilder {
    ElementBuilder::new("title")
}
pub fn base() -> ElementBuilder {
    ElementBuilder::new("base")
}
pub fn link() -> ElementBuilder {
    ElementBuilder::new("link")
}
pub fn meta() -> ElementBuilder {
    ElementBuilder::new("meta")
}
pub fn style() -> ElementBuilder {
    ElementBuilder::new("style")
}

// Sectioning Root
pub fn body() -> ElementBuilder {
    ElementBuilder::new("body")
}

// Content Sectioning
pub fn address() -> ElementBuilder {
    ElementBuilder::new("address")
}
pub fn article() -> ElementBuilder {
    ElementBuilder::new("article")
}
pub fn aside() -> ElementBuilder {
    ElementBuilder::new("aside")
}
pub fn footer() -> ElementBuilder {
    ElementBuilder::new("footer")
}
pub fn header() -> ElementBuilder {
    ElementBuilder::new("header")
}
pub fn h1() -> ElementBuilder {
    ElementBuilder::new("h1")
}
pub fn h2() -> ElementBuilder {
    ElementBuilder::new("h2")
}
pub fn h3() -> ElementBuilder {
    ElementBuilder::new("h3")
}
pub fn h4() -> ElementBuilder {
    ElementBuilder::new("h4")
}
pub fn h5() -> ElementBuilder {
    ElementBuilder::new("h5")
}
pub fn h6() -> ElementBuilder {
    ElementBuilder::new("h6")
}
pub fn main() -> ElementBuilder {
    ElementBuilder::new("main")
}
pub fn nav() -> ElementBuilder {
    ElementBuilder::new("nav")
}
pub fn section() -> ElementBuilder {
    ElementBuilder::new("section")
}
pub fn hgroup() -> ElementBuilder {
    ElementBuilder::new("hgroup")
}

// Text Content
pub fn blockquote() -> ElementBuilder {
    ElementBuilder::new("blockquote")
}
pub fn dd() -> ElementBuilder {
    ElementBuilder::new("dd")
}
pub fn div() -> ElementBuilder {
    ElementBuilder::new("div")
}
pub fn dl() -> ElementBuilder {
    ElementBuilder::new("dl")
}
pub fn dt() -> ElementBuilder {
    ElementBuilder::new("dt")
}
pub fn figcaption() -> ElementBuilder {
    ElementBuilder::new("figcaption")
}
pub fn figure() -> ElementBuilder {
    ElementBuilder::new("figure")
}
pub fn hr() -> ElementBuilder {
    ElementBuilder::new("hr")
}
pub fn li() -> ElementBuilder {
    ElementBuilder::new("li")
}
pub fn ol() -> ElementBuilder {
    ElementBuilder::new("ol")
}
pub fn p() -> ElementBuilder {
    ElementBuilder::new("p")
}
pub fn pre() -> ElementBuilder {
    ElementBuilder::new("pre")
}
pub fn ul() -> ElementBuilder {
    ElementBuilder::new("ul")
}
pub fn menu() -> ElementBuilder {
    ElementBuilder::new("menu")
}

// Inline Text Semantics
pub fn a() -> ElementBuilder {
    ElementBuilder::new("a")
}
pub fn abbr() -> ElementBuilder {
    ElementBuilder::new("abbr")
}
pub fn b() -> ElementBuilder {
    ElementBuilder::new("b")
}
pub fn bdi() -> ElementBuilder {
    ElementBuilder::new("bdi")
}
pub fn bdo() -> ElementBuilder {
    ElementBuilder::new("bdo")
}
pub fn br() -> ElementBuilder {
    ElementBuilder::new("br")
}
pub fn cite() -> ElementBuilder {
    ElementBuilder::new("cite")
}
pub fn code() -> ElementBuilder {
    ElementBuilder::new("code")
}
pub fn data() -> ElementBuilder {
    ElementBuilder::new("data")
}
pub fn dfn() -> ElementBuilder {
    ElementBuilder::new("dfn")
}
pub fn em() -> ElementBuilder {
    ElementBuilder::new("em")
}
pub fn i() -> ElementBuilder {
    ElementBuilder::new("i")
}
pub fn kbd() -> ElementBuilder {
    ElementBuilder::new("kbd")
}
pub fn mark() -> ElementBuilder {
    ElementBuilder::new("mark")
}
pub fn q() -> ElementBuilder {
    ElementBuilder::new("q")
}
pub fn rp() -> ElementBuilder {
    ElementBuilder::new("rp")
}
pub fn rt() -> ElementBuilder {
    ElementBuilder::new("rt")
}
pub fn ruby() -> ElementBuilder {
    ElementBuilder::new("ruby")
}
pub fn s() -> ElementBuilder {
    ElementBuilder::new("s")
}
pub fn samp() -> ElementBuilder {
    ElementBuilder::new("samp")
}
pub fn small() -> ElementBuilder {
    ElementBuilder::new("small")
}
pub fn span() -> ElementBuilder {
    ElementBuilder::new("span")
}
pub fn strong() -> ElementBuilder {
    ElementBuilder::new("strong")
}
pub fn sub() -> ElementBuilder {
    ElementBuilder::new("sub")
}
pub fn sup() -> ElementBuilder {
    ElementBuilder::new("sup")
}
pub fn time() -> ElementBuilder {
    ElementBuilder::new("time")
}
pub fn u() -> ElementBuilder {
    ElementBuilder::new("u")
}
pub fn var() -> ElementBuilder {
    ElementBuilder::new("var")
}
pub fn wbr() -> ElementBuilder {
    ElementBuilder::new("wbr")
}

// Image and Multimedia
pub fn area() -> ElementBuilder {
    ElementBuilder::new("area")
}
pub fn audio() -> ElementBuilder {
    ElementBuilder::new("audio")
}
pub fn img() -> ElementBuilder {
    ElementBuilder::new("img")
}
pub fn map() -> ElementBuilder {
    ElementBuilder::new("map")
}
pub fn track() -> ElementBuilder {
    ElementBuilder::new("track")
}
pub fn video() -> ElementBuilder {
    ElementBuilder::new("video")
}

// Embedded Content
pub fn embed() -> ElementBuilder {
    ElementBuilder::new("embed")
}
pub fn iframe() -> ElementBuilder {
    ElementBuilder::new("iframe")
}
pub fn object() -> ElementBuilder {
    ElementBuilder::new("object")
}
pub fn param() -> ElementBuilder {
    ElementBuilder::new("param")
}
pub fn picture() -> ElementBuilder {
    ElementBuilder::new("picture")
}
pub fn portal() -> ElementBuilder {
    ElementBuilder::new("portal")
}
pub fn source() -> ElementBuilder {
    ElementBuilder::new("source")
}

// SVG and MathML (with namespace)
const SVG_NS: &str = "http://www.w3.org/2000/svg";

pub fn svg() -> ElementBuilder {
    ElementBuilder::new_with_namespace("svg", SVG_NS)
}
pub fn math() -> ElementBuilder {
    ElementBuilder::new_with_namespace("math", "http://www.w3.org/1998/Math/MathML")
}

// SVG Shape Elements
pub fn circle() -> ElementBuilder {
    ElementBuilder::new_with_namespace("circle", SVG_NS)
}
pub fn ellipse() -> ElementBuilder {
    ElementBuilder::new_with_namespace("ellipse", SVG_NS)
}
pub fn line() -> ElementBuilder {
    ElementBuilder::new_with_namespace("line", SVG_NS)
}
pub fn path() -> ElementBuilder {
    ElementBuilder::new_with_namespace("path", SVG_NS)
}
pub fn polygon() -> ElementBuilder {
    ElementBuilder::new_with_namespace("polygon", SVG_NS)
}
pub fn polyline() -> ElementBuilder {
    ElementBuilder::new_with_namespace("polyline", SVG_NS)
}
pub fn rect() -> ElementBuilder {
    ElementBuilder::new_with_namespace("rect", SVG_NS)
}

// SVG Container Elements
pub fn g() -> ElementBuilder {
    ElementBuilder::new_with_namespace("g", SVG_NS)
}
pub fn defs() -> ElementBuilder {
    ElementBuilder::new_with_namespace("defs", SVG_NS)
}
pub fn symbol() -> ElementBuilder {
    ElementBuilder::new_with_namespace("symbol", SVG_NS)
}
pub fn r#use() -> ElementBuilder {
    ElementBuilder::new_with_namespace("use", SVG_NS)
}

// SVG Text Elements
pub fn text_svg() -> ElementBuilder {
    ElementBuilder::new_with_namespace("text", SVG_NS)
}
pub fn tspan() -> ElementBuilder {
    ElementBuilder::new_with_namespace("tspan", SVG_NS)
}
#[allow(non_snake_case)]
pub fn textPath() -> ElementBuilder {
    ElementBuilder::new_with_namespace("textPath", SVG_NS)
}

// SVG Clipping and Masking
#[allow(non_snake_case)]
pub fn clipPath() -> ElementBuilder {
    ElementBuilder::new_with_namespace("clipPath", SVG_NS)
}
pub fn mask() -> ElementBuilder {
    ElementBuilder::new_with_namespace("mask", SVG_NS)
}

// SVG Gradients and Patterns
#[allow(non_snake_case)]
pub fn linearGradient() -> ElementBuilder {
    ElementBuilder::new_with_namespace("linearGradient", SVG_NS)
}
#[allow(non_snake_case)]
pub fn radialGradient() -> ElementBuilder {
    ElementBuilder::new_with_namespace("radialGradient", SVG_NS)
}
pub fn stop() -> ElementBuilder {
    ElementBuilder::new_with_namespace("stop", SVG_NS)
}
pub fn pattern() -> ElementBuilder {
    ElementBuilder::new_with_namespace("pattern", SVG_NS)
}

// SVG Filter Elements
pub fn filter() -> ElementBuilder {
    ElementBuilder::new_with_namespace("filter", SVG_NS)
}
#[allow(non_snake_case)]
pub fn feGaussianBlur() -> ElementBuilder {
    ElementBuilder::new_with_namespace("feGaussianBlur", SVG_NS)
}
#[allow(non_snake_case)]
pub fn feColorMatrix() -> ElementBuilder {
    ElementBuilder::new_with_namespace("feColorMatrix", SVG_NS)
}
#[allow(non_snake_case)]
pub fn feBlend() -> ElementBuilder {
    ElementBuilder::new_with_namespace("feBlend", SVG_NS)
}
#[allow(non_snake_case)]
pub fn feOffset() -> ElementBuilder {
    ElementBuilder::new_with_namespace("feOffset", SVG_NS)
}
#[allow(non_snake_case)]
pub fn feDropShadow() -> ElementBuilder {
    ElementBuilder::new_with_namespace("feDropShadow", SVG_NS)
}

// SVG Animation Elements
pub fn animate() -> ElementBuilder {
    ElementBuilder::new_with_namespace("animate", SVG_NS)
}
#[allow(non_snake_case)]
pub fn animateTransform() -> ElementBuilder {
    ElementBuilder::new_with_namespace("animateTransform", SVG_NS)
}
#[allow(non_snake_case)]
pub fn animateMotion() -> ElementBuilder {
    ElementBuilder::new_with_namespace("animateMotion", SVG_NS)
}

// SVG Other Elements
pub fn image() -> ElementBuilder {
    ElementBuilder::new_with_namespace("image", SVG_NS)
}
#[allow(non_snake_case)]
pub fn foreignObject() -> ElementBuilder {
    ElementBuilder::new_with_namespace("foreignObject", SVG_NS)
}
pub fn marker() -> ElementBuilder {
    ElementBuilder::new_with_namespace("marker", SVG_NS)
}

// Scripting
pub fn canvas() -> ElementBuilder {
    ElementBuilder::new("canvas")
}
pub fn noscript() -> ElementBuilder {
    ElementBuilder::new("noscript")
}
pub fn script() -> ElementBuilder {
    ElementBuilder::new("script")
}

// Demarcating Edits
pub fn del() -> ElementBuilder {
    ElementBuilder::new("del")
}
pub fn ins() -> ElementBuilder {
    ElementBuilder::new("ins")
}

// Table Content
pub fn caption() -> ElementBuilder {
    ElementBuilder::new("caption")
}
pub fn col() -> ElementBuilder {
    ElementBuilder::new("col")
}
pub fn colgroup() -> ElementBuilder {
    ElementBuilder::new("colgroup")
}
pub fn table() -> ElementBuilder {
    ElementBuilder::new("table")
}
pub fn tbody() -> ElementBuilder {
    ElementBuilder::new("tbody")
}
pub fn td() -> ElementBuilder {
    ElementBuilder::new("td")
}
pub fn tfoot() -> ElementBuilder {
    ElementBuilder::new("tfoot")
}
pub fn th() -> ElementBuilder {
    ElementBuilder::new("th")
}
pub fn thead() -> ElementBuilder {
    ElementBuilder::new("thead")
}
pub fn tr() -> ElementBuilder {
    ElementBuilder::new("tr")
}

// Forms
pub fn button() -> ElementBuilder {
    ElementBuilder::new("button")
}
pub fn datalist() -> ElementBuilder {
    ElementBuilder::new("datalist")
}
pub fn fieldset() -> ElementBuilder {
    ElementBuilder::new("fieldset")
}
pub fn form() -> ElementBuilder {
    ElementBuilder::new("form")
}
pub fn input() -> ElementBuilder {
    ElementBuilder::new("input")
}
pub fn label() -> ElementBuilder {
    ElementBuilder::new("label")
}
pub fn legend() -> ElementBuilder {
    ElementBuilder::new("legend")
}
pub fn meter() -> ElementBuilder {
    ElementBuilder::new("meter")
}
pub fn optgroup() -> ElementBuilder {
    ElementBuilder::new("optgroup")
}
pub fn option() -> ElementBuilder {
    ElementBuilder::new("option")
}
pub fn output() -> ElementBuilder {
    ElementBuilder::new("output")
}
pub fn progress() -> ElementBuilder {
    ElementBuilder::new("progress")
}
pub fn select() -> ElementBuilder {
    ElementBuilder::new("select")
}
pub fn textarea() -> ElementBuilder {
    ElementBuilder::new("textarea")
}

// Interactive Elements
pub fn details() -> ElementBuilder {
    ElementBuilder::new("details")
}
pub fn dialog() -> ElementBuilder {
    ElementBuilder::new("dialog")
}
pub fn summary() -> ElementBuilder {
    ElementBuilder::new("summary")
}

// Web Components
pub fn slot() -> ElementBuilder {
    ElementBuilder::new("slot")
}
pub fn template() -> ElementBuilder {
    ElementBuilder::new("template")
}

// =============================================================================
// Macros for Const Static Text
// =============================================================================

/// Add static text to an element builder with compile-time const verification.
///
/// This macro ensures the string literal is evaluated in a const context,
/// guaranteeing it will be embedded in the template and skip diffing.
///
/// # Example
/// ```rust,ignore
/// use dioxus::prelude::*;
///
/// div()
///     .pipe(static_str!("Hello, "))    // Guaranteed const
///     .child(user_name)                 // Dynamic
///     .pipe(static_str!("!"))           // Guaranteed const
///     .build()
/// ```
///
/// Or use the extension trait method:
/// ```rust,ignore
/// use dioxus::prelude::*;
///
/// let builder = div();
/// static_str!(builder, "Hello, World!")
///     .build()
/// ```
#[macro_export]
macro_rules! static_str {
    ($builder:expr, $text:literal) => {{
        const TEXT: &'static str = $text;
        $builder.static_text(TEXT)
    }};
    ($text:literal) => {{
        const TEXT: &'static str = $text;
        |builder: $crate::builder::ElementBuilder| builder.static_text(TEXT)
    }};
}

/// Pipe helper trait for using closures with builders.
pub trait BuilderExt: Sized {
    /// Apply a function to this builder.
    fn pipe<F, R>(self, f: F) -> R
    where
        F: FnOnce(Self) -> R;
}

impl BuilderExt for ElementBuilder {
    fn pipe<F, R>(self, f: F) -> R
    where
        F: FnOnce(Self) -> R,
    {
        f(self)
    }
}

/// Macro for setting data-* attributes with automatic prefix.
///
/// # Example
/// ```rust,ignore
/// use dioxus::prelude::*;
///
/// let builder = div();
/// data!(builder, "testid", "my-element")
///     .pipe(|b| data!(b, "count", "5"))
///     .build()
/// // Results in: data-testid="my-element" data-count="5"
/// ```
#[macro_export]
macro_rules! data {
    ($builder:expr, $suffix:literal, $value:expr) => {{
        const NAME: &'static str = concat!("data-", $suffix);
        $builder.push_attribute(NAME, None, $value, false)
    }};
}
