use crate::innerlude::VProps;
use crate::{any_props::BoxedAnyProps, innerlude::ScopeState};
use crate::{arena::ElementId, Element, Event};
use crate::{
    innerlude::{ElementRef, EventHandler, MountId},
    properties::ComponentFunction,
};
use crate::{Properties, VirtualDom};
use core::panic;
use std::ops::Deref;
use std::rc::Rc;
use std::vec;
use std::{
    any::{Any, TypeId},
    cell::Cell,
    fmt::{Arguments, Debug},
};

pub type TemplateId = &'static str;

/// The actual state of the component's most recent computation
///
/// Because Dioxus accepts components in the form of `async fn() -> Result<VNode>`, we need to support both
/// sync and async versions.
///
/// Dioxus will do its best to immediately resolve any async components into a regular Element, but as an implementor
/// you might need to handle the case where there's no node immediately ready.
pub enum RenderReturn {
    /// A currently-available element
    Ready(VNode),

    /// The component aborted rendering early. It might've thrown an error.
    ///
    /// In its place we've produced a placeholder to locate its spot in the dom when it recovers.
    Aborted(VNode),
}

impl Clone for RenderReturn {
    fn clone(&self) -> Self {
        match self {
            RenderReturn::Ready(node) => RenderReturn::Ready(node.clone_mounted()),
            RenderReturn::Aborted(node) => RenderReturn::Aborted(node.clone_mounted()),
        }
    }
}

impl Default for RenderReturn {
    fn default() -> Self {
        RenderReturn::Aborted(VNode::placeholder())
    }
}

impl Deref for RenderReturn {
    type Target = VNode;

    fn deref(&self) -> &Self::Target {
        match self {
            RenderReturn::Ready(node) | RenderReturn::Aborted(node) => node,
        }
    }
}

/// The information about the
#[derive(Debug)]
pub(crate) struct VNodeMount {
    /// The parent of this node
    pub parent: Option<ElementRef>,

    /// A back link to the original node
    pub node: VNode,

    /// The IDs for the roots of this template - to be used when moving the template around and removing it from
    /// the actual Dom
    pub root_ids: Box<[ElementId]>,

    /// The element in the DOM that each attribute is mounted to
    pub(crate) mounted_attributes: Box<[ElementId]>,

    /// For components: This is the ScopeId the component is mounted to
    /// For other dynamic nodes: This is element in the DOM that each dynamic node is mounted to
    pub(crate) mounted_dynamic_nodes: Box<[usize]>,
}

/// A reference to a template along with any context needed to hydrate it
///
/// The dynamic parts of the template are stored separately from the static parts. This allows faster diffing by skipping
/// static parts of the template.
#[derive(Debug)]
pub struct VNodeInner {
    /// The key given to the root of this template.
    ///
    /// In fragments, this is the key of the first child. In other cases, it is the key of the root.
    pub key: Option<String>,

    /// The static nodes and static descriptor of the template
    pub template: Cell<Template>,

    /// The dynamic nodes in the template
    pub dynamic_nodes: Box<[DynamicNode]>,

    /// The dynamic attribute slots in the template
    ///
    /// This is a list of positions in the template where dynamic attributes can be inserted.
    ///
    /// The inner list *must* be in the format [static named attributes, remaining dynamically named attributes].
    ///
    /// For example:
    /// ```rust, ignore
    /// div {
    ///     class: "{class}",
    ///     ..attrs,
    ///     p {
    ///         color: "{color}",
    ///     }
    /// }
    /// ```
    ///
    /// Would be represented as:
    /// ```rust, ignore
    /// [
    ///     [class, every attribute in attrs sorted by name], // Slot 0 in the template
    ///     [color], // Slot 1 in the template
    /// ]
    /// ```
    pub dynamic_attrs: Box<[Box<[Attribute]>]>,
}

/// A reference to a template along with any context needed to hydrate it
///
/// The dynamic parts of the template are stored separately from the static parts. This allows faster diffing by skipping
/// static parts of the template.
#[derive(Debug)]
pub struct VNode {
    vnode: Rc<VNodeInner>,

    /// The mount information for this template
    pub(crate) mount: Cell<MountId>,
}

impl Clone for VNode {
    fn clone(&self) -> Self {
        Self {
            vnode: self.vnode.clone(),
            mount: Default::default(),
        }
    }
}

impl PartialEq for VNode {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.vnode, &other.vnode)
    }
}

impl Deref for VNode {
    type Target = VNodeInner;

    fn deref(&self) -> &Self::Target {
        &self.vnode
    }
}

impl VNode {
    /// Clone the element while retaining the mount information of the node
    pub(crate) fn clone_mounted(&self) -> Self {
        Self {
            vnode: self.vnode.clone(),
            mount: self.mount.clone(),
        }
    }

    /// Create a template with no nodes that will be skipped over during diffing
    pub fn empty() -> Element {
        Some(Self {
            vnode: Rc::new(VNodeInner {
                key: None,
                dynamic_nodes: Box::new([]),
                dynamic_attrs: Box::new([]),
                template: Cell::new(Template {
                    name: "packages/core/nodes.rs:180:0:0",
                    roots: &[],
                    node_paths: &[],
                    attr_paths: &[],
                }),
            }),
            mount: Default::default(),
        })
    }

    /// Create a template with a single placeholder node
    pub fn placeholder() -> Self {
        Self {
            vnode: Rc::new(VNodeInner {
                key: None,
                dynamic_nodes: Box::new([DynamicNode::Placeholder(Default::default())]),
                dynamic_attrs: Box::new([]),
                template: Cell::new(Template {
                    name: "packages/core/nodes.rs:198:0:0",
                    roots: &[TemplateNode::Dynamic { id: 0 }],
                    node_paths: &[&[]],
                    attr_paths: &[],
                }),
            }),
            mount: Default::default(),
        }
    }

    /// Create a new VNode
    pub fn new(
        key: Option<String>,
        template: Template,
        dynamic_nodes: Box<[DynamicNode]>,
        dynamic_attrs: Box<[Box<[Attribute]>]>,
    ) -> Self {
        Self {
            vnode: Rc::new(VNodeInner {
                key,
                template: Cell::new(template),
                dynamic_nodes,
                dynamic_attrs,
            }),
            mount: Default::default(),
        }
    }

    /// Load a dynamic root at the given index
    ///
    /// Returns [`None`] if the root is actually a static node (Element/Text)
    pub fn dynamic_root(&self, idx: usize) -> Option<&DynamicNode> {
        match &self.template.get().roots[idx] {
            TemplateNode::Element { .. } | TemplateNode::Text { text: _ } => None,
            TemplateNode::Dynamic { id } | TemplateNode::DynamicText { id } => {
                Some(&self.dynamic_nodes[*id])
            }
        }
    }

    /// Get the mounted id for a dynamic node index
    pub fn mounted_dynamic_node(
        &self,
        dynamic_node_idx: usize,
        dom: &VirtualDom,
    ) -> Option<ElementId> {
        let mount = self.mount.get().as_usize()?;

        match &self.dynamic_nodes[dynamic_node_idx] {
            DynamicNode::Text(_) | DynamicNode::Placeholder(_) => dom
                .mounts
                .get(mount)?
                .mounted_dynamic_nodes
                .get(dynamic_node_idx)
                .map(|id| ElementId(*id)),
            _ => None,
        }
    }

    /// Get the mounted id for a root node index
    pub fn mounted_root(&self, root_idx: usize, dom: &VirtualDom) -> Option<ElementId> {
        let mount = self.mount.get().as_usize()?;

        dom.mounts.get(mount)?.root_ids.get(root_idx).copied()
    }

    /// Get the mounted id for a dynamic attribute index
    pub fn mounted_dynamic_attribute(
        &self,
        dynamic_attribute_idx: usize,
        dom: &VirtualDom,
    ) -> Option<ElementId> {
        let mount = self.mount.get().as_usize()?;

        dom.mounts
            .get(mount)?
            .mounted_attributes
            .get(dynamic_attribute_idx)
            .copied()
    }
}

/// A static layout of a UI tree that describes a set of dynamic and static nodes.
///
/// This is the core innovation in Dioxus. Most UIs are made of static nodes, yet participate in diffing like any
/// dynamic node. This struct can be created at compile time. It promises that its name is unique, allow Dioxus to use
/// its static description of the UI to skip immediately to the dynamic nodes during diffing.
///
/// For this to work properly, the [`Template::name`] *must* be unique across your entire project. This can be done via variety of
/// ways, with the suggested approach being the unique code location (file, line, col, etc).
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Hash, Eq, PartialOrd, Ord)]
pub struct Template {
    /// The name of the template. This must be unique across your entire program for template diffing to work properly
    ///
    /// If two templates have the same name, it's likely that Dioxus will panic when diffing.
    #[cfg_attr(
        feature = "serialize",
        serde(deserialize_with = "deserialize_string_leaky")
    )]
    pub name: &'static str,

    /// The list of template nodes that make up the template
    ///
    /// Unlike react, calls to `rsx!` can have multiple roots. This list supports that paradigm.
    #[cfg_attr(feature = "serialize", serde(deserialize_with = "deserialize_leaky"))]
    pub roots: &'static [TemplateNode],

    /// The paths of each node relative to the root of the template.
    ///
    /// These will be one segment shorter than the path sent to the renderer since those paths are relative to the
    /// topmost element, not the `roots` field.
    #[cfg_attr(
        feature = "serialize",
        serde(deserialize_with = "deserialize_bytes_leaky")
    )]
    pub node_paths: &'static [&'static [u8]],

    /// The paths of each dynamic attribute relative to the root of the template
    ///
    /// These will be one segment shorter than the path sent to the renderer since those paths are relative to the
    /// topmost element, not the `roots` field.
    #[cfg_attr(
        feature = "serialize",
        serde(deserialize_with = "deserialize_bytes_leaky")
    )]
    pub attr_paths: &'static [&'static [u8]],
}

#[cfg(feature = "serialize")]
fn deserialize_string_leaky<'a, 'de, D>(deserializer: D) -> Result<&'a str, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    let deserialized = String::deserialize(deserializer)?;
    Ok(&*Box::leak(deserialized.into_boxed_str()))
}

#[cfg(feature = "serialize")]
fn deserialize_bytes_leaky<'a, 'de, D>(deserializer: D) -> Result<&'a [&'a [u8]], D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    let deserialized = Vec::<Vec<u8>>::deserialize(deserializer)?;
    let deserialized = deserialized
        .into_iter()
        .map(|v| &*Box::leak(v.into_boxed_slice()))
        .collect::<Vec<_>>();
    Ok(&*Box::leak(deserialized.into_boxed_slice()))
}

#[cfg(feature = "serialize")]
fn deserialize_leaky<'a, 'de, T: serde::Deserialize<'de>, D>(
    deserializer: D,
) -> Result<&'a [T], D::Error>
where
    T: serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    let deserialized = Box::<[T]>::deserialize(deserializer)?;
    Ok(&*Box::leak(deserialized))
}

#[cfg(feature = "serialize")]
fn deserialize_option_leaky<'a, 'de, D>(deserializer: D) -> Result<Option<&'static str>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    let deserialized = Option::<String>::deserialize(deserializer)?;
    Ok(deserialized.map(|deserialized| &*Box::leak(deserialized.into_boxed_str())))
}

impl Template {
    /// Is this template worth caching at all, since it's completely runtime?
    ///
    /// There's no point in saving templates that are completely dynamic, since they'll be recreated every time anyway.
    pub fn is_completely_dynamic(&self) -> bool {
        use TemplateNode::*;
        self.roots
            .iter()
            .all(|root| matches!(root, Dynamic { .. } | DynamicText { .. }))
    }
}

/// A statically known node in a layout.
///
/// This can be created at compile time, saving the VirtualDom time when diffing the tree
#[derive(Debug, Clone, Copy, PartialEq, Hash, Eq, PartialOrd, Ord)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "type")
)]
pub enum TemplateNode {
    /// An statically known element in the dom.
    ///
    /// In HTML this would be something like `<div id="123"> </div>`
    Element {
        /// The name of the element
        ///
        /// IE for a div, it would be the string "div"
        tag: &'static str,

        /// The namespace of the element
        ///
        /// In HTML, this would be a valid URI that defines a namespace for all elements below it
        /// SVG is an example of this namespace
        #[cfg_attr(
            feature = "serialize",
            serde(deserialize_with = "deserialize_option_leaky")
        )]
        namespace: Option<&'static str>,

        /// A list of possibly dynamic attributes for this element
        ///
        /// An attribute on a DOM node, such as `id="my-thing"` or `href="https://example.com"`.
        #[cfg_attr(feature = "serialize", serde(deserialize_with = "deserialize_leaky"))]
        attrs: &'static [TemplateAttribute],

        /// A list of template nodes that define another set of template nodes
        #[cfg_attr(feature = "serialize", serde(deserialize_with = "deserialize_leaky"))]
        children: &'static [TemplateNode],
    },

    /// This template node is just a piece of static text
    Text {
        /// The actual text
        text: &'static str,
    },

    /// This template node is unknown, and needs to be created at runtime.
    Dynamic {
        /// The index of the dynamic node in the VNode's dynamic_nodes list
        id: usize,
    },

    /// This template node is known to be some text, but needs to be created at runtime
    ///
    /// This is separate from the pure Dynamic variant for various optimizations
    DynamicText {
        /// The index of the dynamic node in the VNode's dynamic_nodes list
        id: usize,
    },
}

impl TemplateNode {
    /// Try to load the dynamic node at the given index
    pub fn dynamic_id(&self) -> Option<usize> {
        use TemplateNode::*;
        match self {
            Dynamic { id } | DynamicText { id } => Some(*id),
            _ => None,
        }
    }
}

/// A node created at runtime
///
/// This node's index in the DynamicNode list on VNode should match its repsective `Dynamic` index
#[derive(Debug)]
pub enum DynamicNode {
    /// A component node
    ///
    /// Most of the time, Dioxus will actually know which component this is as compile time, but the props and
    /// assigned scope are dynamic.
    ///
    /// The actual VComponent can be dynamic between two VNodes, though, allowing implementations to swap
    /// the render function at runtime
    Component(VComponent),

    /// A text node
    Text(VText),

    /// A placeholder
    ///
    /// Used by suspense when a node isn't ready and by fragments that don't render anything
    ///
    /// In code, this is just an ElementId whose initial value is set to 0 upon creation
    Placeholder(VPlaceholder),

    /// A list of VNodes.
    ///
    /// Note that this is not a list of dynamic nodes. These must be VNodes and created through conditional rendering
    /// or iterators.
    Fragment(Vec<VNode>),
}

impl DynamicNode {
    /// Convert any item that implements [`IntoDynNode`] into a [`DynamicNode`]
    pub fn make_node<'c, I>(into: impl IntoDynNode<I> + 'c) -> DynamicNode {
        into.into_dyn_node()
    }
}

impl Default for DynamicNode {
    fn default() -> Self {
        Self::Placeholder(Default::default())
    }
}

/// An instance of a child component
pub struct VComponent {
    /// The name of this component
    pub name: &'static str,

    /// The function pointer of the component, known at compile time
    ///
    /// It is possible that components get folded at compile time, so these shouldn't be really used as a key
    pub(crate) render_fn: TypeId,

    pub(crate) props: BoxedAnyProps,
}

impl VComponent {
    /// Create a new [`VComponent`] variant
    pub fn new<P, M: 'static>(
        component: impl ComponentFunction<P, M>,
        props: P,
        fn_name: &'static str,
    ) -> Self
    where
        P: Properties + 'static,
    {
        let render_fn = component.id();
        let props = Box::new(VProps::new(
            component,
            <P as Properties>::memoize,
            props,
            fn_name,
        ));

        VComponent {
            name: fn_name,
            props,
            render_fn,
        }
    }

    /// Get the scope this node is mounted to if it's mounted
    ///
    /// This is useful for rendering nodes outside of the VirtualDom, such as in SSR
    ///
    /// Returns [`None`] if the node is not mounted
    pub fn mounted_scope<'a>(
        &self,
        dynamic_node_index: usize,
        vnode: &VNode,
        dom: &'a VirtualDom,
    ) -> Option<&'a ScopeState> {
        let mount = vnode.mount.get().as_usize()?;

        let scope_id = dom.mounts.get(mount)?.mounted_dynamic_nodes[dynamic_node_index];

        dom.scopes.get(scope_id)
    }
}

impl std::fmt::Debug for VComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VComponent")
            .field("name", &self.name)
            .finish()
    }
}

/// A text node
#[derive(Clone, Debug)]
pub struct VText {
    /// The actual text itself
    pub value: String,
}

impl VText {
    /// Create a new VText
    pub fn new(value: String) -> Self {
        Self { value }
    }
}

impl From<Arguments<'_>> for VText {
    fn from(args: Arguments) -> Self {
        Self::new(args.to_string())
    }
}

/// A placeholder node, used by suspense and fragments
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct VPlaceholder {}

/// An attribute of the TemplateNode, created at compile time
#[derive(Debug, PartialEq, Hash, Eq, PartialOrd, Ord)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "type")
)]
pub enum TemplateAttribute {
    /// This attribute is entirely known at compile time, enabling
    Static {
        /// The name of this attribute.
        ///
        /// For example, the `href` attribute in `href="https://example.com"`, would have the name "href"
        name: &'static str,

        /// The value of this attribute, known at compile time
        ///
        /// Currently this only accepts &str, so values, even if they're known at compile time, are not known
        value: &'static str,

        /// The namespace of this attribute. Does not exist in the HTML spec
        namespace: Option<&'static str>,
    },

    /// The attribute in this position is actually determined dynamically at runtime
    ///
    /// This is the index into the dynamic_attributes field on the container VNode
    Dynamic {
        /// The index
        id: usize,
    },
}

/// An attribute on a DOM node, such as `id="my-thing"` or `href="https://example.com"`
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    /// The name of the attribute.
    pub name: &'static str,

    /// The value of the attribute
    pub value: AttributeValue,

    /// The namespace of the attribute.
    ///
    /// Doesn’t exist in the html spec. Used in Dioxus to denote “style” tags and other attribute groups.
    pub namespace: Option<&'static str>,

    /// An indication of we should always try and set the attribute. Used in controlled components to ensure changes are propagated
    pub volatile: bool,
}

impl Attribute {
    /// Create a new [`Attribute`] from a name, value, namespace, and volatile bool
    ///
    /// "Volatile" referes to whether or not Dioxus should always override the value. This helps prevent the UI in
    /// some renderers stay in sync with the VirtualDom's understanding of the world
    pub fn new(
        name: &'static str,
        value: impl IntoAttributeValue,
        namespace: Option<&'static str>,
        volatile: bool,
    ) -> Attribute {
        Attribute {
            name,
            namespace,
            volatile,
            value: value.into_value(),
        }
    }
}

/// Any of the built-in values that the Dioxus VirtualDom supports as dynamic attributes on elements
///
/// These are built-in to be faster during the diffing process. To use a custom value, use the [`AttributeValue::Any`]
/// variant.
pub enum AttributeValue {
    /// Text attribute
    Text(String),

    /// A float
    Float(f64),

    /// Signed integer
    Int(i64),

    /// Boolean
    Bool(bool),

    /// A listener, like "onclick"
    Listener(ListenerCb),

    /// An arbitrary value that implements PartialEq and is static
    Any(Box<dyn AnyValue>),

    /// A "none" value, resulting in the removal of an attribute from the dom
    None,
}

impl AttributeValue {
    /// Create a new [`AttributeValue`] with the listener variant from a callback
    ///
    /// The callback must be confined to the lifetime of the ScopeState
    pub fn listener<T: 'static>(mut callback: impl FnMut(Event<T>) + 'static) -> AttributeValue {
        AttributeValue::Listener(EventHandler::new(move |event: Event<dyn Any>| {
            let data = event.data.downcast::<T>().unwrap();
            // if let Ok(data) = event.data.downcast::<T>() {
            callback(Event {
                propagates: event.propagates,
                data,
            });
            // }
        }))
    }

    /// Create a new [`AttributeValue`] with a value that implements [`AnyValue`]
    pub fn any_value<T: AnyValue>(value: T) -> AttributeValue {
        AttributeValue::Any(Box::new(value))
    }
}

pub type ListenerCb = EventHandler<Event<dyn Any>>;

impl std::fmt::Debug for AttributeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text(arg0) => f.debug_tuple("Text").field(arg0).finish(),
            Self::Float(arg0) => f.debug_tuple("Float").field(arg0).finish(),
            Self::Int(arg0) => f.debug_tuple("Int").field(arg0).finish(),
            Self::Bool(arg0) => f.debug_tuple("Bool").field(arg0).finish(),
            Self::Listener(_) => f.debug_tuple("Listener").finish(),
            Self::Any(_) => f.debug_tuple("Any").finish(),
            Self::None => write!(f, "None"),
        }
    }
}

impl PartialEq for AttributeValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Text(l0), Self::Text(r0)) => l0 == r0,
            (Self::Float(l0), Self::Float(r0)) => l0 == r0,
            (Self::Int(l0), Self::Int(r0)) => l0 == r0,
            (Self::Bool(l0), Self::Bool(r0)) => l0 == r0,
            (Self::Listener(_), Self::Listener(_)) => true,
            (Self::Any(l0), Self::Any(r0)) => l0.as_ref().any_cmp(r0.as_ref()),
            (Self::None, Self::None) => true,
            _ => false,
        }
    }
}

impl Clone for AttributeValue {
    fn clone(&self) -> Self {
        match self {
            Self::Text(arg0) => Self::Text(arg0.clone()),
            Self::Float(arg0) => Self::Float(*arg0),
            Self::Int(arg0) => Self::Int(*arg0),
            Self::Bool(arg0) => Self::Bool(*arg0),
            Self::Listener(_) | Self::Any(_) => panic!("Cannot clone listener or any value"),
            Self::None => Self::None,
        }
    }
}

#[doc(hidden)]
pub trait AnyValue: 'static {
    fn any_cmp(&self, other: &dyn AnyValue) -> bool;
    fn as_any(&self) -> &dyn Any;
    fn type_id(&self) -> TypeId {
        self.as_any().type_id()
    }
}

impl<T: Any + PartialEq + 'static> AnyValue for T {
    fn any_cmp(&self, other: &dyn AnyValue) -> bool {
        if let Some(other) = other.as_any().downcast_ref() {
            self == other
        } else {
            false
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A trait that allows various items to be converted into a dynamic node for the rsx macro
pub trait IntoDynNode<A = ()> {
    /// Consume this item along with a scopestate and produce a DynamicNode
    ///
    /// You can use the bump alloactor of the scopestate to creat the dynamic node
    fn into_dyn_node(self) -> DynamicNode;
}

impl IntoDynNode for () {
    fn into_dyn_node(self) -> DynamicNode {
        DynamicNode::default()
    }
}
impl IntoDynNode for VNode {
    fn into_dyn_node(self) -> DynamicNode {
        DynamicNode::Fragment(vec![self])
    }
}

impl IntoDynNode for DynamicNode {
    fn into_dyn_node(self) -> DynamicNode {
        self
    }
}

impl<T: IntoDynNode> IntoDynNode for Option<T> {
    fn into_dyn_node(self) -> DynamicNode {
        match self {
            Some(val) => val.into_dyn_node(),
            None => DynamicNode::default(),
        }
    }
}

impl IntoDynNode for &Element {
    fn into_dyn_node(self) -> DynamicNode {
        match self.as_ref() {
            Some(val) => val.clone().into_dyn_node(),
            _ => DynamicNode::default(),
        }
    }
}

impl IntoDynNode for &str {
    fn into_dyn_node(self) -> DynamicNode {
        DynamicNode::Text(VText {
            value: self.to_string(),
        })
    }
}

impl IntoDynNode for String {
    fn into_dyn_node(self) -> DynamicNode {
        DynamicNode::Text(VText { value: self })
    }
}

impl IntoDynNode for Arguments<'_> {
    fn into_dyn_node(self) -> DynamicNode {
        DynamicNode::Text(VText {
            value: self.to_string(),
        })
    }
}

impl IntoDynNode for &VNode {
    fn into_dyn_node(self) -> DynamicNode {
        DynamicNode::Fragment(vec![self.clone()])
    }
}

pub trait IntoVNode {
    fn into_vnode(self) -> VNode;
}
impl IntoVNode for VNode {
    fn into_vnode(self) -> VNode {
        self
    }
}
impl IntoVNode for &VNode {
    fn into_vnode(self) -> VNode {
        self.clone()
    }
}
impl IntoVNode for Element {
    fn into_vnode(self) -> VNode {
        match self {
            Some(val) => val.into_vnode(),
            _ => VNode::empty().unwrap(),
        }
    }
}
impl IntoVNode for &Element {
    fn into_vnode(self) -> VNode {
        match self {
            Some(val) => val.into_vnode(),
            _ => VNode::empty().unwrap(),
        }
    }
}

// Note that we're using the E as a generic but this is never crafted anyways.
pub struct FromNodeIterator;
impl<T, I> IntoDynNode<FromNodeIterator> for T
where
    T: Iterator<Item = I>,
    I: IntoVNode,
{
    fn into_dyn_node(self) -> DynamicNode {
        let children: Vec<_> = self.into_iter().map(|node| node.into_vnode()).collect();

        if children.is_empty() {
            DynamicNode::default()
        } else {
            DynamicNode::Fragment(children)
        }
    }
}

/// A value that can be converted into an attribute value
pub trait IntoAttributeValue {
    /// Convert into an attribute value
    fn into_value(self) -> AttributeValue;
}

impl IntoAttributeValue for AttributeValue {
    fn into_value(self) -> AttributeValue {
        self
    }
}

impl IntoAttributeValue for &str {
    fn into_value(self) -> AttributeValue {
        AttributeValue::Text(self.to_string())
    }
}

impl IntoAttributeValue for String {
    fn into_value(self) -> AttributeValue {
        AttributeValue::Text(self)
    }
}

impl IntoAttributeValue for f64 {
    fn into_value(self) -> AttributeValue {
        AttributeValue::Float(self)
    }
}

impl IntoAttributeValue for i64 {
    fn into_value(self) -> AttributeValue {
        AttributeValue::Int(self)
    }
}

impl IntoAttributeValue for bool {
    fn into_value(self) -> AttributeValue {
        AttributeValue::Bool(self)
    }
}

impl IntoAttributeValue for Arguments<'_> {
    fn into_value(self) -> AttributeValue {
        AttributeValue::Text(self.to_string())
    }
}

impl IntoAttributeValue for Box<dyn AnyValue> {
    fn into_value(self) -> AttributeValue {
        AttributeValue::Any(self)
    }
}

impl<T: IntoAttributeValue> IntoAttributeValue for Option<T> {
    fn into_value(self) -> AttributeValue {
        match self {
            Some(val) => val.into_value(),
            None => AttributeValue::None,
        }
    }
}

/// A trait for anything that has a dynamic list of attributes
pub trait HasAttributes {
    /// Push an attribute onto the list of attributes
    fn push_attribute(
        self,
        name: &'static str,
        ns: Option<&'static str>,
        attr: impl IntoAttributeValue,
        volatile: bool,
    ) -> Self;
}
