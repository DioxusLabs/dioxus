use crate::innerlude::{ElementRef, VNodeId};
use crate::{
    any_props::AnyProps, arena::ElementId, Element, Event, LazyNodes, ScopeId, ScopeState,
};
use bumpalo::boxed::Box as BumpBox;
use bumpalo::Bump;
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    fmt::{Arguments, Debug},
};

pub type TemplateId = &'static str;

/// The actual state of the component's most recent computation
///
/// Because Dioxus accepts components in the form of `async fn(Scope) -> Result<VNode>`, we need to support both
/// sync and async versions.
///
/// Dioxus will do its best to immediately resolve any async components into a regular Element, but as an implementor
/// you might need to handle the case where there's no node immediately ready.
pub enum RenderReturn<'a> {
    /// A currently-available element
    Ready(VNode<'a>),

    /// The component aborted rendering early. It might've thrown an error.
    ///
    /// In its place we've produced a placeholder to locate its spot in the dom when
    /// it recovers.
    Aborted(VPlaceholder),
}

impl<'a> Default for RenderReturn<'a> {
    fn default() -> Self {
        RenderReturn::Aborted(VPlaceholder::default())
    }
}

/// A reference to a template along with any context needed to hydrate it
///
/// The dynamic parts of the template are stored separately from the static parts. This allows faster diffing by skipping
/// static parts of the template.
#[derive(Debug, Clone)]
pub struct VNode<'a> {
    /// The key given to the root of this template.
    ///
    /// In fragments, this is the key of the first child. In other cases, it is the key of the root.
    pub key: Option<&'a str>,

    /// When rendered, this template will be linked to its parent manually
    pub(crate) parent: Cell<Option<ElementRef>>,

    /// The bubble id assigned to the child that we need to update and drop when diffing happens
    pub(crate) stable_id: Cell<Option<VNodeId>>,

    /// The static nodes and static descriptor of the template
    pub template: Cell<Template<'static>>,

    /// The IDs for the roots of this template - to be used when moving the template around and removing it from
    /// the actual Dom
    pub root_ids: RefCell<bumpalo::collections::Vec<'a, ElementId>>,

    /// The dynamic parts of the template
    pub dynamic_nodes: &'a [DynamicNode<'a>],

    /// The dynamic parts of the template
    pub dynamic_attrs: &'a [Attribute<'a>],
}

impl<'a> VNode<'a> {
    /// Create a template with no nodes that will be skipped over during diffing
    pub fn empty(cx: &'a ScopeState) -> Element<'a> {
        Some(VNode {
            key: None,
            parent: Default::default(),
            stable_id: Default::default(),
            root_ids: RefCell::new(bumpalo::collections::Vec::new_in(cx.bump())),
            dynamic_nodes: &[],
            dynamic_attrs: &[],
            template: Cell::new(Template {
                name: "dioxus-empty",
                roots: &[],
                node_paths: &[],
                attr_paths: &[],
            }),
        })
    }

    /// Create a new VNode
    pub fn new(
        key: Option<&'a str>,
        template: Template<'static>,
        root_ids: bumpalo::collections::Vec<'a, ElementId>,
        dynamic_nodes: &'a [DynamicNode<'a>],
        dynamic_attrs: &'a [Attribute<'a>],
    ) -> Self {
        Self {
            key,
            parent: Cell::new(None),
            stable_id: Cell::new(None),
            template: Cell::new(template),
            root_ids: RefCell::new(root_ids),
            dynamic_nodes,
            dynamic_attrs,
        }
    }

    /// Get the stable id of this node used for bubbling events
    pub(crate) fn stable_id(&self) -> Option<VNodeId> {
        self.stable_id.get()
    }

    /// Load a dynamic root at the given index
    ///
    /// Returns [`None`] if the root is actually a static node (Element/Text)
    pub fn dynamic_root(&self, idx: usize) -> Option<&'a DynamicNode<'a>> {
        match &self.template.get().roots[idx] {
            TemplateNode::Element { .. } | TemplateNode::Text { text: _ } => None,
            TemplateNode::Dynamic { id } | TemplateNode::DynamicText { id } => {
                Some(&self.dynamic_nodes[*id])
            }
        }
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
pub struct Template<'a> {
    /// The name of the template. This must be unique across your entire program for template diffing to work properly
    ///
    /// If two templates have the same name, it's likely that Dioxus will panic when diffing.
    #[cfg_attr(
        feature = "serialize",
        serde(deserialize_with = "deserialize_string_leaky")
    )]
    pub name: &'a str,

    /// The list of template nodes that make up the template
    ///
    /// Unlike react, calls to `rsx!` can have multiple roots. This list supports that paradigm.
    #[cfg_attr(feature = "serialize", serde(deserialize_with = "deserialize_leaky"))]
    pub roots: &'a [TemplateNode<'a>],

    /// The paths of each node relative to the root of the template.
    ///
    /// These will be one segment shorter than the path sent to the renderer since those paths are relative to the
    /// topmost element, not the `roots` field.
    #[cfg_attr(
        feature = "serialize",
        serde(deserialize_with = "deserialize_bytes_leaky")
    )]
    pub node_paths: &'a [&'a [u8]],

    /// The paths of each dynamic attribute relative to the root of the template
    ///
    /// These will be one segment shorter than the path sent to the renderer since those paths are relative to the
    /// topmost element, not the `roots` field.
    #[cfg_attr(
        feature = "serialize",
        serde(deserialize_with = "deserialize_bytes_leaky")
    )]
    pub attr_paths: &'a [&'a [u8]],
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

impl<'a> Template<'a> {
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
pub enum TemplateNode<'a> {
    /// An statically known element in the dom.
    ///
    /// In HTML this would be something like `<div id="123"> </div>`
    Element {
        /// The name of the element
        ///
        /// IE for a div, it would be the string "div"
        tag: &'a str,

        /// The namespace of the element
        ///
        /// In HTML, this would be a valid URI that defines a namespace for all elements below it
        /// SVG is an example of this namespace
        #[cfg_attr(
            feature = "serialize",
            serde(deserialize_with = "deserialize_option_leaky")
        )]
        namespace: Option<&'a str>,

        /// A list of possibly dynamic attribues for this element
        ///
        /// An attribute on a DOM node, such as `id="my-thing"` or `href="https://example.com"`.
        #[cfg_attr(feature = "serialize", serde(deserialize_with = "deserialize_leaky"))]
        attrs: &'a [TemplateAttribute<'a>],

        /// A list of template nodes that define another set of template nodes
        #[cfg_attr(feature = "serialize", serde(deserialize_with = "deserialize_leaky"))]
        children: &'a [TemplateNode<'a>],
    },

    /// This template node is just a piece of static text
    Text {
        /// The actual text
        text: &'a str,
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

/// A node created at runtime
///
/// This node's index in the DynamicNode list on VNode should match its repsective `Dynamic` index
#[derive(Debug)]
pub enum DynamicNode<'a> {
    /// A component node
    ///
    /// Most of the time, Dioxus will actually know which component this is as compile time, but the props and
    /// assigned scope are dynamic.
    ///
    /// The actual VComponent can be dynamic between two VNodes, though, allowing implementations to swap
    /// the render function at runtime
    Component(VComponent<'a>),

    /// A text node
    Text(VText<'a>),

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
    Fragment(&'a [VNode<'a>]),
}

impl Default for DynamicNode<'_> {
    fn default() -> Self {
        Self::Placeholder(Default::default())
    }
}

/// An instance of a child component
pub struct VComponent<'a> {
    /// The name of this component
    pub name: &'static str,

    /// Are the props valid for the 'static lifetime?
    ///
    /// Internally, this is used as a guarantee. Externally, this might be incorrect, so don't count on it.
    ///
    /// This flag is assumed by the [`crate::Properties`] trait which is unsafe to implement
    pub(crate) static_props: bool,

    /// The assigned Scope for this component
    pub(crate) scope: Cell<Option<ScopeId>>,

    /// The function pointer of the component, known at compile time
    ///
    /// It is possible that components get folded at compile time, so these shouldn't be really used as a key
    pub(crate) render_fn: *const (),

    pub(crate) props: RefCell<Option<Box<dyn AnyProps<'a> + 'a>>>,
}

impl<'a> VComponent<'a> {
    /// Get the scope that this component is mounted to
    pub fn mounted_scope(&self) -> Option<ScopeId> {
        self.scope.get()
    }
}

impl<'a> std::fmt::Debug for VComponent<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VComponent")
            .field("name", &self.name)
            .field("static_props", &self.static_props)
            .field("scope", &self.scope)
            .finish()
    }
}

/// An instance of some text, mounted to the DOM
#[derive(Debug)]
pub struct VText<'a> {
    /// The actual text itself
    pub value: &'a str,

    /// The ID of this node in the real DOM
    pub(crate) id: Cell<Option<ElementId>>,
}

impl<'a> VText<'a> {
    /// Create a new VText
    pub fn new(value: &'a str) -> Self {
        Self {
            value,
            id: Default::default(),
        }
    }

    /// Get the mounted ID of this node
    pub fn mounted_element(&self) -> Option<ElementId> {
        self.id.get()
    }
}

/// A placeholder node, used by suspense and fragments
#[derive(Debug, Default)]
pub struct VPlaceholder {
    /// The ID of this node in the real DOM
    pub(crate) id: Cell<Option<ElementId>>,
    /// The parent of this node
    pub(crate) parent: Cell<Option<ElementRef>>,
}

impl VPlaceholder {
    /// Get the mounted ID of this node
    pub fn mounted_element(&self) -> Option<ElementId> {
        self.id.get()
    }
}

/// An attribute of the TemplateNode, created at compile time
#[derive(Debug, PartialEq, Hash, Eq, PartialOrd, Ord)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "type")
)]
pub enum TemplateAttribute<'a> {
    /// This attribute is entirely known at compile time, enabling
    Static {
        /// The name of this attribute.
        ///
        /// For example, the `href` attribute in `href="https://example.com"`, would have the name "href"
        name: &'a str,

        /// The value of this attribute, known at compile time
        ///
        /// Currently this only accepts &str, so values, even if they're known at compile time, are not known
        value: &'a str,

        /// The namespace of this attribute. Does not exist in the HTML spec
        namespace: Option<&'a str>,
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
#[derive(Debug)]
pub struct Attribute<'a> {
    /// The name of the attribute.
    pub name: &'a str,

    /// The value of the attribute
    pub value: AttributeValue<'a>,

    /// The namespace of the attribute.
    ///
    /// Doesn’t exist in the html spec. Used in Dioxus to denote “style” tags and other attribute groups.
    pub namespace: Option<&'static str>,

    /// An indication of we should always try and set the attribute. Used in controlled components to ensure changes are propagated
    pub volatile: bool,

    /// The element in the DOM that this attribute belongs to
    pub(crate) mounted_element: Cell<ElementId>,
}

impl<'a> Attribute<'a> {
    /// Create a new attribute
    pub fn new(
        name: &'a str,
        value: AttributeValue<'a>,
        namespace: Option<&'static str>,
        volatile: bool,
    ) -> Self {
        Self {
            name,
            value,
            namespace,
            volatile,
            mounted_element: Cell::new(ElementId::default()),
        }
    }

    /// Get the element that this attribute is mounted to
    pub fn mounted_element(&self) -> ElementId {
        self.mounted_element.get()
    }
}

/// Any of the built-in values that the Dioxus VirtualDom supports as dynamic attributes on elements
///
/// These are built-in to be faster during the diffing process. To use a custom value, use the [`AttributeValue::Any`]
/// variant.
pub enum AttributeValue<'a> {
    /// Text attribute
    Text(&'a str),

    /// A float
    Float(f64),

    /// Signed integer
    Int(i64),

    /// Boolean
    Bool(bool),

    /// A listener, like "onclick"
    Listener(RefCell<Option<ListenerCb<'a>>>),

    /// An arbitrary value that implements PartialEq and is static
    Any(RefCell<Option<BumpBox<'a, dyn AnyValue>>>),

    /// A "none" value, resulting in the removal of an attribute from the dom
    None,
}

pub type ListenerCb<'a> = BumpBox<'a, dyn FnMut(Event<dyn Any>) + 'a>;

/// Any of the built-in values that the Dioxus VirtualDom supports as dynamic attributes on elements that are borrowed
///
/// These varients are used to communicate what the value of an attribute is that needs to be updated
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialize", serde(untagged))]
pub enum BorrowedAttributeValue<'a> {
    /// Text attribute
    Text(&'a str),

    /// A float
    Float(f64),

    /// Signed integer
    Int(i64),

    /// Boolean
    Bool(bool),

    /// An arbitrary value that implements PartialEq and is static
    #[cfg_attr(
        feature = "serialize",
        serde(
            deserialize_with = "deserialize_any_value",
            serialize_with = "serialize_any_value"
        )
    )]
    Any(std::cell::Ref<'a, dyn AnyValue>),

    /// A "none" value, resulting in the removal of an attribute from the dom
    None,
}

impl<'a> From<&'a AttributeValue<'a>> for BorrowedAttributeValue<'a> {
    fn from(value: &'a AttributeValue<'a>) -> Self {
        match value {
            AttributeValue::Text(value) => BorrowedAttributeValue::Text(value),
            AttributeValue::Float(value) => BorrowedAttributeValue::Float(*value),
            AttributeValue::Int(value) => BorrowedAttributeValue::Int(*value),
            AttributeValue::Bool(value) => BorrowedAttributeValue::Bool(*value),
            AttributeValue::Listener(_) => {
                panic!("A listener cannot be turned into a borrowed value")
            }
            AttributeValue::Any(value) => {
                let value = value.borrow();
                BorrowedAttributeValue::Any(std::cell::Ref::map(value, |value| {
                    &**value.as_ref().unwrap()
                }))
            }
            AttributeValue::None => BorrowedAttributeValue::None,
        }
    }
}

impl Debug for BorrowedAttributeValue<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text(arg0) => f.debug_tuple("Text").field(arg0).finish(),
            Self::Float(arg0) => f.debug_tuple("Float").field(arg0).finish(),
            Self::Int(arg0) => f.debug_tuple("Int").field(arg0).finish(),
            Self::Bool(arg0) => f.debug_tuple("Bool").field(arg0).finish(),
            Self::Any(_) => f.debug_tuple("Any").field(&"...").finish(),
            Self::None => write!(f, "None"),
        }
    }
}

impl PartialEq for BorrowedAttributeValue<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Text(l0), Self::Text(r0)) => l0 == r0,
            (Self::Float(l0), Self::Float(r0)) => l0 == r0,
            (Self::Int(l0), Self::Int(r0)) => l0 == r0,
            (Self::Bool(l0), Self::Bool(r0)) => l0 == r0,
            (Self::Any(l0), Self::Any(r0)) => l0.any_cmp(&**r0),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

#[cfg(feature = "serialize")]
fn serialize_any_value<S>(_: &std::cell::Ref<'_, dyn AnyValue>, _: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    panic!("Any cannot be serialized")
}

#[cfg(feature = "serialize")]
fn deserialize_any_value<'de, 'a, D>(_: D) -> Result<std::cell::Ref<'a, dyn AnyValue>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    panic!("Any cannot be deserialized")
}

impl<'a> std::fmt::Debug for AttributeValue<'a> {
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

impl<'a> PartialEq for AttributeValue<'a> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Text(l0), Self::Text(r0)) => l0 == r0,
            (Self::Float(l0), Self::Float(r0)) => l0 == r0,
            (Self::Int(l0), Self::Int(r0)) => l0 == r0,
            (Self::Bool(l0), Self::Bool(r0)) => l0 == r0,
            (Self::Listener(_), Self::Listener(_)) => true,
            (Self::Any(l0), Self::Any(r0)) => {
                let l0 = l0.borrow();
                let r0 = r0.borrow();
                l0.as_ref().unwrap().any_cmp(&**r0.as_ref().unwrap())
            }
            _ => false,
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

impl<'a> RenderReturn<'a> {
    pub(crate) unsafe fn extend_lifetime_ref<'c>(&self) -> &'c RenderReturn<'c> {
        unsafe { std::mem::transmute(self) }
    }
    pub(crate) unsafe fn extend_lifetime<'c>(self) -> RenderReturn<'c> {
        unsafe { std::mem::transmute(self) }
    }
}

/// A trait that allows various items to be converted into a dynamic node for the rsx macro
pub trait IntoDynNode<'a, A = ()> {
    /// Consume this item along with a scopestate and produce a DynamicNode
    ///
    /// You can use the bump alloactor of the scopestate to creat the dynamic node
    fn into_vnode(self, cx: &'a ScopeState) -> DynamicNode<'a>;
}

impl<'a> IntoDynNode<'a> for () {
    fn into_vnode(self, _cx: &'a ScopeState) -> DynamicNode<'a> {
        DynamicNode::default()
    }
}
impl<'a> IntoDynNode<'a> for VNode<'a> {
    fn into_vnode(self, _cx: &'a ScopeState) -> DynamicNode<'a> {
        DynamicNode::Fragment(_cx.bump().alloc([self]))
    }
}

impl<'a> IntoDynNode<'a> for DynamicNode<'a> {
    fn into_vnode(self, _cx: &'a ScopeState) -> DynamicNode<'a> {
        self
    }
}

impl<'a, T: IntoDynNode<'a>> IntoDynNode<'a> for Option<T> {
    fn into_vnode(self, _cx: &'a ScopeState) -> DynamicNode<'a> {
        match self {
            Some(val) => val.into_vnode(_cx),
            None => DynamicNode::default(),
        }
    }
}

impl<'a> IntoDynNode<'a> for &Element<'a> {
    fn into_vnode(self, _cx: &'a ScopeState) -> DynamicNode<'a> {
        match self.as_ref() {
            Some(val) => val.clone().into_vnode(_cx),
            _ => DynamicNode::default(),
        }
    }
}

impl<'a, 'b> IntoDynNode<'a> for LazyNodes<'a, 'b> {
    fn into_vnode(self, cx: &'a ScopeState) -> DynamicNode<'a> {
        DynamicNode::Fragment(cx.bump().alloc([cx.render(self).unwrap()]))
    }
}

impl<'a, 'b> IntoDynNode<'b> for &'a str {
    fn into_vnode(self, cx: &'b ScopeState) -> DynamicNode<'b> {
        DynamicNode::Text(VText {
            value: cx.bump().alloc_str(self),
            id: Default::default(),
        })
    }
}

impl IntoDynNode<'_> for String {
    fn into_vnode(self, cx: &ScopeState) -> DynamicNode {
        DynamicNode::Text(VText {
            value: cx.bump().alloc_str(&self),
            id: Default::default(),
        })
    }
}

impl<'b> IntoDynNode<'b> for Arguments<'_> {
    fn into_vnode(self, cx: &'b ScopeState) -> DynamicNode<'b> {
        cx.text_node(self)
    }
}

impl<'a> IntoDynNode<'a> for &'a VNode<'a> {
    fn into_vnode(self, _cx: &'a ScopeState) -> DynamicNode<'a> {
        DynamicNode::Fragment(_cx.bump().alloc([VNode {
            parent: self.parent.clone(),
            stable_id: self.stable_id.clone(),
            template: self.template.clone(),
            root_ids: self.root_ids.clone(),
            key: self.key,
            dynamic_nodes: self.dynamic_nodes,
            dynamic_attrs: self.dynamic_attrs,
        }]))
    }
}

pub trait IntoTemplate<'a> {
    fn into_template(self, _cx: &'a ScopeState) -> VNode<'a>;
}
impl<'a> IntoTemplate<'a> for VNode<'a> {
    fn into_template(self, _cx: &'a ScopeState) -> VNode<'a> {
        self
    }
}
impl<'a> IntoTemplate<'a> for Element<'a> {
    fn into_template(self, cx: &'a ScopeState) -> VNode<'a> {
        match self {
            Some(val) => val.into_template(cx),
            _ => VNode::empty(cx).unwrap(),
        }
    }
}
impl<'a, 'b> IntoTemplate<'a> for LazyNodes<'a, 'b> {
    fn into_template(self, cx: &'a ScopeState) -> VNode<'a> {
        cx.render(self).unwrap()
    }
}

// Note that we're using the E as a generic but this is never crafted anyways.
pub struct FromNodeIterator;
impl<'a, T, I> IntoDynNode<'a, FromNodeIterator> for T
where
    T: Iterator<Item = I>,
    I: IntoTemplate<'a>,
{
    fn into_vnode(self, cx: &'a ScopeState) -> DynamicNode<'a> {
        let mut nodes = bumpalo::collections::Vec::new_in(cx.bump());

        nodes.extend(self.into_iter().map(|node| node.into_template(cx)));

        match nodes.into_bump_slice() {
            children if children.is_empty() => DynamicNode::default(),
            children => DynamicNode::Fragment(children),
        }
    }
}

/// A value that can be converted into an attribute value
pub trait IntoAttributeValue<'a> {
    /// Convert into an attribute value
    fn into_value(self, bump: &'a Bump) -> AttributeValue<'a>;
}

impl<'a> IntoAttributeValue<'a> for AttributeValue<'a> {
    fn into_value(self, _: &'a Bump) -> AttributeValue<'a> {
        self
    }
}

impl<'a> IntoAttributeValue<'a> for &'a str {
    fn into_value(self, _: &'a Bump) -> AttributeValue<'a> {
        AttributeValue::Text(self)
    }
}

impl<'a> IntoAttributeValue<'a> for String {
    fn into_value(self, cx: &'a Bump) -> AttributeValue<'a> {
        AttributeValue::Text(cx.alloc_str(&self))
    }
}

impl<'a> IntoAttributeValue<'a> for f64 {
    fn into_value(self, _: &'a Bump) -> AttributeValue<'a> {
        AttributeValue::Float(self)
    }
}

impl<'a> IntoAttributeValue<'a> for i64 {
    fn into_value(self, _: &'a Bump) -> AttributeValue<'a> {
        AttributeValue::Int(self)
    }
}

impl<'a> IntoAttributeValue<'a> for bool {
    fn into_value(self, _: &'a Bump) -> AttributeValue<'a> {
        AttributeValue::Bool(self)
    }
}

impl<'a> IntoAttributeValue<'a> for Arguments<'_> {
    fn into_value(self, bump: &'a Bump) -> AttributeValue<'a> {
        use bumpalo::core_alloc::fmt::Write;
        let mut str_buf = bumpalo::collections::String::new_in(bump);
        str_buf.write_fmt(self).unwrap();
        AttributeValue::Text(str_buf.into_bump_str())
    }
}

impl<'a> IntoAttributeValue<'a> for BumpBox<'a, dyn AnyValue> {
    fn into_value(self, _: &'a Bump) -> AttributeValue<'a> {
        AttributeValue::Any(RefCell::new(Some(self)))
    }
}

impl<'a, T: IntoAttributeValue<'a>> IntoAttributeValue<'a> for Option<T> {
    fn into_value(self, bump: &'a Bump) -> AttributeValue<'a> {
        match self {
            Some(val) => val.into_value(bump),
            None => AttributeValue::None,
        }
    }
}
