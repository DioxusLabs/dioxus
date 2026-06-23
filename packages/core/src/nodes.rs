use crate::{
    DynamicAnchor, DynamicAttrSlot, DynamicNodeSlot, Element, Event, Properties, ScopeId, Template,
    VirtualDom,
    arena::ElementId,
    events::ListenerCallback,
    innerlude::{BoxedAnyProps, MountId, ScopeState, VProps},
    properties::ComponentFunction,
};
use dioxus_core_template::{TemplateRawTree, TemplateStorage};
use dioxus_core_types::DioxusFormattable;

use std::ops::Deref;
use std::rc::Rc;
use std::{
    any::{Any, TypeId},
    fmt::{Arguments, Debug},
};

/// Runtime node and attribute values that hydrate a static [`Template`].
pub struct DynamicValues {
    /// Root key for this render.
    pub(crate) key: Option<String>,

    /// Dynamic node values in template order.
    pub(crate) dynamic_nodes: Vec<DynamicNode>,

    /// Dynamic attribute values in template order.
    pub(crate) dynamic_attrs: Vec<Box<[Attribute]>>,
}

impl Debug for DynamicValues {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynamicValues").finish_non_exhaustive()
    }
}

impl DynamicValues {
    /// Create a dynamic node/attribute payload.
    ///
    /// Each dynamic attribute slot is normalized so its attributes are sorted by
    /// `(name, namespace)`. Duplicate keys keep their relative input order, giving
    /// last-wins semantics for spread attributes like `..props.attributes`.
    #[inline]
    pub fn from_parts(
        key: Option<String>,
        dynamic_nodes: Box<[DynamicNode]>,
        dynamic_attrs: Box<[Box<[Attribute]>]>,
    ) -> Self {
        let mut values = Self {
            key,
            dynamic_nodes: dynamic_nodes.into_vec(),
            dynamic_attrs: dynamic_attrs.into_vec(),
        };
        values.normalize();
        values
    }

    /// Create an empty dynamic node/attribute payload.
    pub(crate) fn new() -> Self {
        Self {
            key: None,
            dynamic_nodes: Vec::new(),
            dynamic_attrs: Vec::new(),
        }
    }

    /// Set the root key for this render.
    pub(crate) fn set_key(&mut self, key: Option<String>) {
        self.key = key;
    }

    /// Push a dynamic node slot.
    pub(crate) fn push_node(&mut self, value: DynamicNode) {
        self.dynamic_nodes.push(value);
    }

    /// Push a dynamic attribute slot.
    ///
    /// Dynamic attribute slots are normalized by [`Self::normalize`] before the values are stored
    /// on a [`VNode`].
    pub(crate) fn push_attrs(&mut self, value: Box<[Attribute]>) {
        self.dynamic_attrs.push(value);
    }

    /// Normalize dynamic attribute slots for diffing.
    #[inline]
    pub(crate) fn normalize(&mut self) {
        for slot in self.dynamic_attrs.iter_mut() {
            if slot.len() > 1 {
                slot.sort_by(|a, b| (a.name, a.namespace).cmp(&(b.name, b.namespace)));
            }
        }
    }

    /// The root key for this render.
    #[inline]
    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }

    /// The dynamic node values in template order.
    #[inline]
    pub fn dynamic_node_values(&self) -> &[DynamicNode] {
        &self.dynamic_nodes
    }

    /// The dynamic attribute values in template order.
    #[inline]
    pub fn dynamic_attr_values(&self) -> &[Box<[Attribute]>] {
        &self.dynamic_attrs
    }
}

/// A static template with the values rendered for it.
pub struct VNodeInner {
    /// The static template.
    pub template: Template,

    /// The rendered dynamic node/attribute values.
    pub view: DynamicValues,
}

impl Debug for VNodeInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VNodeInner").finish_non_exhaustive()
    }
}

impl Deref for VNodeInner {
    type Target = DynamicValues;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

/// A reference to a template along with any context needed to hydrate it
///
/// The dynamic parts of the template are stored separately from the static parts. This allows faster diffing by skipping
/// static parts of the template.
#[derive(Clone)]
pub struct VNode {
    vnode: Rc<VNodeInner>,
}

impl Debug for VNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VNode").finish_non_exhaustive()
    }
}

impl Default for VNode {
    fn default() -> Self {
        Self::placeholder()
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
    /// Create a template with no nodes that will be skipped over during diffing
    pub fn empty() -> Element {
        Ok(Self::default())
    }

    /// Create an empty VNode that produces no DOM nodes
    pub fn placeholder() -> Self {
        use std::cell::OnceCell;
        // We can reuse this empty vnode across the same thread to save memory
        thread_local! {
            static EMPTY_VNODE: OnceCell<Rc<VNodeInner>> = const { OnceCell::new() };
        }
        static TREE: TemplateRawTree = TemplateRawTree::DynamicNode;
        static STORAGE: TemplateStorage<1, 1, 1> = TemplateStorage::build_from_tree(&TREE);
        static EMPTY_TEMPLATE: Template = STORAGE.as_template();

        let vnode = EMPTY_VNODE.with(|cell| {
            cell.get_or_init(move || {
                Rc::new(VNodeInner {
                    template: EMPTY_TEMPLATE,
                    view: DynamicValues::from_parts(
                        None,
                        Box::new([DynamicNode::Fragment(Vec::new())]),
                        Box::new([]),
                    ),
                })
            })
            .clone()
        });
        Self { vnode }
    }

    /// Create a VNode that represents a failed component render (suspense / error boundary).
    /// Unlike [`Self::placeholder`], this contributes a single empty text anchor to the DOM so
    /// that the parent boundary's diff has a stable slot to replace once content resolves.
    pub(crate) fn error_anchor() -> Self {
        use std::cell::OnceCell;
        thread_local! {
            static ERROR_ANCHOR_VNODE: OnceCell<Rc<VNodeInner>> = const { OnceCell::new() };
        }
        static TREE: TemplateRawTree = TemplateRawTree::DynamicNode;
        static STORAGE: TemplateStorage<1, 1, 1> = TemplateStorage::build_from_tree(&TREE);
        static ERROR_ANCHOR_TEMPLATE: Template = STORAGE.as_template();

        let vnode = ERROR_ANCHOR_VNODE.with(|cell| {
            cell.get_or_init(move || {
                Rc::new(VNodeInner {
                    template: ERROR_ANCHOR_TEMPLATE,
                    view: DynamicValues::from_parts(
                        None,
                        Box::new([DynamicNode::Text(VText {
                            value: String::new(),
                        })]),
                        Box::new([]),
                    ),
                })
            })
            .clone()
        });
        Self { vnode }
    }

    /// Create a new VNode from a static template and dynamic node/attribute payload.
    pub fn new(template: Template, mut values: DynamicValues) -> Self {
        values.normalize();

        debug_assert!(
            values.dynamic_nodes.len()
                == template
                    .anchors()
                    .iter()
                    .map(|anchor| anchor.nodes().end)
                    .max()
                    .unwrap_or_default(),
            "bad dynamic node count"
        );
        debug_assert!(
            values.dynamic_attrs.len()
                == template
                    .anchors()
                    .iter()
                    .map(|anchor| anchor.attributes().end)
                    .max()
                    .unwrap_or_default(),
            "bad dynamic attribute count"
        );

        // Dynamic attribute slots are required to be sorted by `(name, namespace)` for the diff.
        // That invariant is established centrally by `DynamicValues::normalize`.

        Self {
            vnode: Rc::new(VNodeInner {
                template,
                view: values,
            }),
        }
    }
}

#[derive(Clone, Copy, Debug)]
/// A [`VNode`] paired with the live mount that renders it.
pub struct MountedVNode<'a> {
    vnode: &'a VNode,
    mount: MountId,
}

impl<'a> MountedVNode<'a> {
    pub(crate) const fn new(vnode: &'a VNode, mount: MountId) -> Self {
        Self { vnode, mount }
    }

    pub(crate) const fn mount(self) -> MountId {
        self.mount
    }

    /// Return the underlying vnode.
    pub const fn vnode(self) -> &'a VNode {
        self.vnode
    }

    /// Get the mounted id for a dynamic node.
    pub fn mounted_dynamic_node(
        self,
        slot: DynamicNodeSlot<'a>,
        dom: &VirtualDom,
    ) -> Option<ElementId> {
        let dynamic_node_idx = slot.index();
        match &self.vnode.dynamic_nodes[dynamic_node_idx] {
            DynamicNode::Text(_) => dom
                .mounted_dynamic_text_node(self.mount, dynamic_node_idx)
                .map(|id| id.element_id()),
            _ => None,
        }
    }

    /// Get the mounted id for a root node.
    pub fn mounted_root(self, root_idx: usize, dom: &VirtualDom) -> Option<ElementId> {
        if root_idx >= dom.mounted_root_count(self.mount) {
            return None;
        }

        dom.mounted_root_node(self.mount, root_idx)
            .map(|id| id.element_id())
    }

    /// Get the mounted id for the static template node addressed by a dynamic anchor.
    pub fn mounted_anchor_node(
        self,
        anchor: DynamicAnchor<'a>,
        dom: &VirtualDom,
    ) -> Option<ElementId> {
        dom.mounted_anchor_node(self.mount, anchor.anchor_index())
            .map(|id| id.element_id())
    }

    /// Get the mounted id for the anchor that owns a dynamic attribute.
    pub fn mounted_dynamic_attribute(
        self,
        slot: DynamicAttrSlot<'a>,
        dom: &VirtualDom,
    ) -> Option<ElementId> {
        self.vnode.dynamic_attrs.get(slot.index())?;
        self.mounted_anchor_node(slot.anchor(), dom)
    }

    /// Get mounted children for a dynamic fragment.
    pub fn mounted_fragment_children(
        self,
        slot: DynamicNodeSlot<'a>,
        dom: &VirtualDom,
    ) -> Vec<MountedVNode<'a>> {
        let dynamic_node_idx = slot.index();
        let DynamicNode::Fragment(children) = &self.vnode.dynamic_nodes[dynamic_node_idx] else {
            return Vec::new();
        };

        children
            .iter()
            .zip(dom.mounted_fragment_children(self.mount, dynamic_node_idx, children.len()))
            .map(|(vnode, mount)| MountedVNode::new(vnode, mount))
            .collect()
    }
}

impl Deref for MountedVNode<'_> {
    type Target = VNode;

    fn deref(&self) -> &Self::Target {
        self.vnode
    }
}

/// A node created at runtime
///
/// This node's index in the DynamicNode list on VNode should match its respective `Dynamic` index
#[derive(Debug, Clone)]
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

    /// A list of VNodes.
    ///
    /// Note that this is not a list of dynamic nodes. These must be VNodes and created through conditional rendering
    /// or iterators. An empty Fragment represents the absence of content at this slot.
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
        Self::Fragment(Vec::new())
    }
}

/// An instance of a child component
pub struct VComponent {
    /// The name of this component
    pub name: &'static str,

    /// The raw pointer to the render function.
    pub(crate) render_fn: usize,

    /// The rendering lifecycle for this component's scope.
    pub(crate) driver: Rc<dyn crate::render_driver::RenderDriver>,

    /// The props this component renders from.
    pub(crate) props: BoxedAnyProps,
}

impl Clone for VComponent {
    fn clone(&self) -> Self {
        Self {
            name: self.name,
            render_fn: self.render_fn,
            driver: self.driver.clone(),
            props: self.props.duplicate(),
        }
    }
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
        let render_fn = component.fn_ptr();
        let props = Box::new(VProps::new(
            component,
            <P as Properties>::memoize,
            props,
            fn_name,
        ));
        Self::new_with_driver(
            fn_name,
            render_fn,
            Rc::new(crate::render_driver::BodyDriver),
            props,
        )
    }

    /// Create a new [`VComponent`] whose scope is rendered by `driver`.
    pub(crate) fn new_with_driver(
        fn_name: &'static str,
        render_fn: usize,
        driver: Rc<dyn crate::render_driver::RenderDriver>,
        props: BoxedAnyProps,
    ) -> Self {
        VComponent {
            name: fn_name,
            render_fn,
            driver,
            props,
        }
    }

    /// Get the [`ScopeId`] this node is mounted to if it's mounted
    ///
    /// This is useful for rendering nodes outside of the VirtualDom, such as in SSR
    ///
    /// Returns [`None`] if the node is not mounted
    pub fn mounted_scope_id(
        &self,
        slot: DynamicNodeSlot<'_>,
        vnode: MountedVNode<'_>,
        dom: &VirtualDom,
    ) -> Option<ScopeId> {
        dom.mounted_dynamic_component_scope(vnode.mount(), slot.index())
    }

    /// Get the scope this node is mounted to if it's mounted
    ///
    /// This is useful for rendering nodes outside of the VirtualDom, such as in SSR
    ///
    /// Returns [`None`] if the node is not mounted
    pub fn mounted_scope<'a>(
        &self,
        slot: DynamicNodeSlot<'_>,
        vnode: MountedVNode<'_>,
        dom: &'a VirtualDom,
    ) -> Option<&'a ScopeState> {
        let scope_id = dom.mounted_dynamic_component_scope(vnode.mount(), slot.index())?;

        dom.scopes.get(scope_id.index())
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
    #[inline]
    pub fn new(value: impl ToString) -> Self {
        Self {
            value: value.to_string(),
        }
    }
}

impl From<Arguments<'_>> for VText {
    fn from(args: Arguments) -> Self {
        Self::new(args.to_string())
    }
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
    /// "Volatile" refers to whether or not Dioxus should always override the value. This helps prevent the UI in
    /// some renderers stay in sync with the VirtualDom's understanding of the world
    pub fn new<T>(
        name: &'static str,
        value: impl IntoAttributeValue<T>,
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
#[derive(Clone)]
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
    Listener(ListenerCallback),

    /// An arbitrary value that implements PartialEq and is static
    Any(Rc<dyn AnyValue>),

    /// A "none" value, resulting in the removal of an attribute from the dom
    None,
}

impl AttributeValue {
    /// Create a new [`AttributeValue`] with the listener variant from a callback
    ///
    /// The callback must be confined to the lifetime of the ScopeState
    pub fn listener<T: 'static>(callback: impl FnMut(Event<T>) + 'static) -> AttributeValue {
        AttributeValue::Listener(ListenerCallback::new(callback).erase())
    }

    /// Create a new [`AttributeValue`] with a value that implements [`AnyValue`]
    pub fn any_value<T: AnyValue>(value: T) -> AttributeValue {
        AttributeValue::Any(Rc::new(value))
    }
}

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
            (Self::Listener(l0), Self::Listener(r0)) => l0 == r0,
            (Self::Any(l0), Self::Any(r0)) => l0.as_ref().any_cmp(r0.as_ref()),
            (Self::None, Self::None) => true,
            _ => false,
        }
    }
}

/// Trait object support for custom [`AttributeValue::Any`] values.
pub trait AnyValue: 'static {
    /// Compare this value with another erased value.
    fn any_cmp(&self, other: &dyn AnyValue) -> bool;

    /// Return this value as [`Any`] for downcasting.
    fn as_any(&self) -> &dyn Any;

    /// Return the underlying value's [`TypeId`].
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
    /// Consume this item and produce a DynamicNode
    fn into_dyn_node(self) -> DynamicNode;
}

impl IntoDynNode for () {
    #[inline]
    fn into_dyn_node(self) -> DynamicNode {
        DynamicNode::default()
    }
}
impl IntoDynNode for VNode {
    #[inline]
    fn into_dyn_node(self) -> DynamicNode {
        DynamicNode::Fragment(vec![self])
    }
}
impl IntoDynNode for DynamicNode {
    #[inline]
    fn into_dyn_node(self) -> DynamicNode {
        self
    }
}
impl<T: IntoDynNode> IntoDynNode for Option<T> {
    #[inline]
    fn into_dyn_node(self) -> DynamicNode {
        match self {
            Some(val) => val.into_dyn_node(),
            None => DynamicNode::default(),
        }
    }
}
impl IntoDynNode for &Element {
    #[inline]
    fn into_dyn_node(self) -> DynamicNode {
        match self.as_ref() {
            Ok(val) => val.into_dyn_node(),
            _ => DynamicNode::default(),
        }
    }
}
impl IntoDynNode for Element {
    #[inline]
    fn into_dyn_node(self) -> DynamicNode {
        match self {
            Ok(val) => val.into_dyn_node(),
            _ => DynamicNode::default(),
        }
    }
}
impl IntoDynNode for &Option<VNode> {
    #[inline]
    fn into_dyn_node(self) -> DynamicNode {
        match self.as_ref() {
            Some(val) => val.clone().into_dyn_node(),
            _ => DynamicNode::default(),
        }
    }
}
impl IntoDynNode for &str {
    #[inline]
    fn into_dyn_node(self) -> DynamicNode {
        DynamicNode::Text(VText {
            value: self.to_string(),
        })
    }
}
impl IntoDynNode for String {
    #[inline]
    fn into_dyn_node(self) -> DynamicNode {
        DynamicNode::Text(VText { value: self })
    }
}
impl IntoDynNode for Arguments<'_> {
    #[inline]
    fn into_dyn_node(self) -> DynamicNode {
        DynamicNode::Text(VText {
            value: self.to_string(),
        })
    }
}
impl IntoDynNode for &VNode {
    #[inline]
    fn into_dyn_node(self) -> DynamicNode {
        DynamicNode::Fragment(vec![self.clone()])
    }
}

/// Convert a value into a [`VNode`].
pub trait IntoVNode {
    /// Convert this value into a [`VNode`].
    fn into_vnode(self) -> VNode;
}
impl IntoVNode for VNode {
    #[inline]
    fn into_vnode(self) -> VNode {
        self
    }
}
impl IntoVNode for &VNode {
    #[inline]
    fn into_vnode(self) -> VNode {
        self.clone()
    }
}
impl IntoVNode for Element {
    #[inline]
    fn into_vnode(self) -> VNode {
        match self {
            Ok(val) => val.into_vnode(),
            _ => VNode::default(),
        }
    }
}
impl IntoVNode for &Element {
    #[inline]
    fn into_vnode(self) -> VNode {
        match self {
            Ok(val) => val.into_vnode(),
            _ => VNode::default(),
        }
    }
}
impl IntoVNode for Option<VNode> {
    #[inline]
    fn into_vnode(self) -> VNode {
        match self {
            Some(val) => val.into_vnode(),
            _ => VNode::default(),
        }
    }
}
impl IntoVNode for &Option<VNode> {
    #[inline]
    fn into_vnode(self) -> VNode {
        match self.as_ref() {
            Some(val) => val.clone().into_vnode(),
            _ => VNode::default(),
        }
    }
}
impl IntoVNode for Option<Element> {
    #[inline]
    fn into_vnode(self) -> VNode {
        match self {
            Some(val) => val.into_vnode(),
            _ => VNode::default(),
        }
    }
}
impl IntoVNode for &Option<Element> {
    #[inline]
    fn into_vnode(self) -> VNode {
        match self.as_ref() {
            Some(val) => val.clone().into_vnode(),
            _ => VNode::default(),
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
    #[inline]
    fn into_dyn_node(self) -> DynamicNode {
        DynamicNode::Fragment(self.into_iter().map(|node| node.into_vnode()).collect())
    }
}

/// A value that can be converted into an attribute value
pub trait IntoAttributeValue<T = ()> {
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

macro_rules! impl_float_attribute_value {
    ($($ty:ty),* $(,)?) => {
        $(
            impl IntoAttributeValue for $ty {
                fn into_value(self) -> AttributeValue {
                    AttributeValue::Float(self as _)
                }
            }
        )*
    };
}

macro_rules! impl_int_attribute_value {
    ($($ty:ty),* $(,)?) => {
        $(
            impl IntoAttributeValue for $ty {
                fn into_value(self) -> AttributeValue {
                    AttributeValue::Int(self as _)
                }
            }
        )*
    };
}

impl_float_attribute_value!(f32, f64);
impl_int_attribute_value!(i8, i16, i32, i64, isize, i128);
impl_int_attribute_value!(u8, u16, u32, u64, usize, u128);

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

impl IntoAttributeValue for Rc<dyn AnyValue> {
    fn into_value(self) -> AttributeValue {
        AttributeValue::Any(self)
    }
}

impl<T> IntoAttributeValue for ListenerCallback<T> {
    fn into_value(self) -> AttributeValue {
        AttributeValue::Listener(self.erase())
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

impl<T: ToOwned<Owned = R>, R: IntoAttributeValue> IntoAttributeValue for &T {
    fn into_value(self) -> AttributeValue {
        self.to_owned().into_value()
    }
}

#[doc(hidden)]
pub struct AnyFmtMarker;
impl<T> IntoAttributeValue<AnyFmtMarker> for T
where
    T: DioxusFormattable,
{
    fn into_value(self) -> AttributeValue {
        AttributeValue::Text(self.format().to_string())
    }
}

/// A trait for anything that has a dynamic list of attributes
pub trait HasAttributes {
    /// Push an attribute onto the list of attributes
    fn push_attribute<T>(
        self,
        name: &'static str,
        ns: Option<&'static str>,
        attr: impl IntoAttributeValue<T>,
        volatile: bool,
    ) -> Self;
}
