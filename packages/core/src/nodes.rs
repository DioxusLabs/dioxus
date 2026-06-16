use crate::{
    Element, Event, Properties, ScopeId, Template, TemplateOp, TemplatePath, VirtualDom,
    arena::ElementId,
    events::ListenerCallback,
    innerlude::{MountId, ScopeState},
    properties::ComponentFunction,
    string_interner::StaticStringInterner,
};
use dioxus_core_types::DioxusFormattable;

use std::ops::Deref;
use std::rc::Rc;
use std::{
    any::{Any, TypeId},
    cell::Cell,
    fmt::{Arguments, Debug},
};

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
    pub template: Template,

    /// The dynamic values in template order.
    ///
    /// Each entry corresponds to one path in [`Template::dynamics`]. Node and attribute slots share
    /// the same index space so the flat template stream can be diffed in a single document-order
    /// pass.
    pub dynamic_values: Box<[DynamicValue]>,
}

/// A reference to a template along with any context needed to hydrate it
///
/// The dynamic parts of the template are stored separately from the static parts. This allows faster diffing by skipping
/// static parts of the template.
#[derive(Debug, Clone)]
pub struct VNode {
    vnode: Rc<VNodeInner>,

    /// The raw mount slot for this template.
    ///
    /// `usize::MAX` means this vnode is not mounted. Convert this raw slot to
    /// `MountId` through `mounted_id` or `unchecked_mounted_id`.
    mount: Cell<usize>,
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
    const UNMOUNTED_MOUNT: usize = usize::MAX;

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
        static EMPTY_TEMPLATE: Template = Template::new(
            &[TemplateOp::text(), TemplateOp::dynamic()],
            StaticStringInterner::empty(),
            &[TemplatePath::root(0).with_appends(true)],
        );
        let vnode = EMPTY_VNODE.with(|cell| {
            cell.get_or_init(move || {
                Rc::new(VNodeInner {
                    key: None,
                    dynamic_values: Box::new([DynamicValue::Node(DynamicNode::Fragment(
                        Vec::new(),
                    ))]),
                    template: EMPTY_TEMPLATE,
                })
            })
            .clone()
        });
        Self {
            vnode,
            mount: Cell::new(Self::UNMOUNTED_MOUNT),
        }
    }

    /// Create a VNode that represents a failed component render (suspense / error boundary).
    /// Unlike [`Self::placeholder`], this contributes a single empty text anchor to the DOM so
    /// that the parent boundary's diff has a stable slot to replace once content resolves.
    pub(crate) fn error_anchor() -> Self {
        use std::cell::OnceCell;
        thread_local! {
            static ERROR_ANCHOR_VNODE: OnceCell<Rc<VNodeInner>> = const { OnceCell::new() };
        }
        static ERROR_ANCHOR_TEMPLATE: Template = Template::new(
            &[TemplateOp::text(), TemplateOp::dynamic()],
            StaticStringInterner::empty(),
            &[TemplatePath::root(0).with_appends(true)],
        );
        let vnode = ERROR_ANCHOR_VNODE.with(|cell| {
            cell.get_or_init(move || {
                Rc::new(VNodeInner {
                    key: None,
                    dynamic_values: Box::new([DynamicValue::Node(DynamicNode::Text(VText {
                        value: String::new(),
                    }))]),
                    template: ERROR_ANCHOR_TEMPLATE,
                })
            })
            .clone()
        });
        Self {
            vnode,
            mount: Cell::new(Self::UNMOUNTED_MOUNT),
        }
    }

    /// Create a new VNode
    #[inline]
    pub fn new(
        key: Option<String>,
        template: Template,
        dynamic_values: Box<[DynamicValue]>,
    ) -> Self {
        // The diff assumes every dynamic attribute slot is sorted by `(name, namespace)`. Named
        // attributes are trivially sorted (one entry per slot); spread attributes are user-provided
        // and the only realistic source of violations.
        #[cfg(debug_assertions)]
        for value in &dynamic_values {
            if let DynamicValue::Attrs(slot) = value {
                for pair in slot.windows(2) {
                    let left = (pair[0].name, pair[0].namespace);
                    let right = (pair[1].name, pair[1].namespace);
                    if left > right {
                        tracing::warn!(
                            "spread attributes in `rsx!` must be sorted by (name, namespace); \
                             found {:?} before {:?}. The diff assumes sorted input and may produce \
                             incorrect updates otherwise.",
                            left,
                            right,
                        );
                        break;
                    }
                }
            }
        }

        Self {
            vnode: Rc::new(VNodeInner {
                key,
                template,
                dynamic_values,
            }),
            mount: Cell::new(Self::UNMOUNTED_MOUNT),
        }
    }

    /// Load a root-level dynamic node slot at the given dynamic node index
    ///
    /// Returns [`None`] if the dynamic node is mounted under a static template node.
    pub fn dynamic_root(&self, idx: usize) -> Option<&DynamicNode> {
        self.template
            .node_paths()
            .any(|(dynamic_idx, path)| dynamic_idx == idx && path.is_root_level_slot())
            .then(|| self.dynamic_values[idx].as_node())
            .flatten()
    }

    /// Get the mount id for this node if it has been mounted.
    pub(crate) fn mounted_id(&self) -> Option<MountId> {
        let mount = self.mount.get();
        (mount != Self::UNMOUNTED_MOUNT).then_some(MountId(mount))
    }

    /// Get the mount id for this node.
    ///
    /// Callers must already know this vnode is mounted.
    pub(crate) fn unchecked_mounted_id(&self) -> MountId {
        MountId(self.mount.get())
    }

    /// Set this node's mount id.
    pub(crate) fn set_mounted_id(&self, mount: MountId) {
        self.mount.set(mount.0);
    }

    /// Take this node's mount id, leaving it unmounted.
    pub(crate) fn take_mounted_id(&self) -> MountId {
        MountId(self.mount.replace(Self::UNMOUNTED_MOUNT))
    }

    /// Clear this node's mount id.
    pub(crate) fn clear_mounted_id(&self) {
        self.mount.set(Self::UNMOUNTED_MOUNT);
    }

    /// Get the mounted id for a dynamic node index
    pub fn mounted_dynamic_node(
        &self,
        dynamic_node_idx: usize,
        dom: &VirtualDom,
    ) -> Option<ElementId> {
        let mount = self.mounted_id()?;

        match self.dynamic_values[dynamic_node_idx].node() {
            DynamicNode::Text(_) => dom
                .mounted_dynamic_text_node(mount, dynamic_node_idx)
                .map(|id| id.element_id()),
            _ => None,
        }
    }

    /// Get the mounted id for a root node index
    pub fn mounted_root(&self, root_idx: usize, dom: &VirtualDom) -> Option<ElementId> {
        let mount = self.mounted_id()?;
        if root_idx >= dom.mounted_root_count(mount) {
            return None;
        }

        dom.mounted_root_node(mount, root_idx)
            .map(|id| id.element_id())
    }

    /// Get the mounted id for a dynamic attribute index
    pub fn mounted_dynamic_attribute(
        &self,
        dynamic_attribute_idx: usize,
        dom: &VirtualDom,
    ) -> Option<ElementId> {
        let mount = self.mounted_id()?;

        dom.mounted_dyn_attr(mount, dynamic_attribute_idx)
            .map(|id| id.element_id())
    }

    /// Create a deep clone of this VNode
    pub(crate) fn deep_clone(&self) -> Self {
        self.deep_clone_inner(false)
    }

    /// Deep-clone the tree while preserving every per-node raw mount slot. Each
    /// `VNodeInner` is freshly allocated so the resulting tree's per-node
    /// `Cell<usize>` slots are independent from this one — diffing against
    /// the clone won't mutate this tree's mount state via the shared `Rc`.
    ///
    /// Used by `SuspenseBranch::root` to hand out a fresh tree per diff pass
    /// without losing the mount info the diff needs to talk to the renderer.
    pub(crate) fn deep_clone_preserving_mounts(&self) -> Self {
        self.deep_clone_inner(true)
    }

    /// Clone this vnode for retained mount lookups.
    ///
    /// Only dynamic fragments contain descendant `VNode` mount cells that can be
    /// invalidated by a later `claim_mount`. Text, attributes, and component
    /// dynamic slots keep their mount state in the mount registry, so a shallow
    /// clone is enough for those templates.
    pub(crate) fn clone_for_mount_snapshot(&self) -> Self {
        if self.needs_deep_mount_snapshot() {
            self.deep_clone_preserving_mounts()
        } else {
            self.clone()
        }
    }

    fn needs_deep_mount_snapshot(&self) -> bool {
        self.vnode.dynamic_values.iter().any(|value| {
            matches!(
                value,
                DynamicValue::Node(DynamicNode::Fragment(nodes)) if !nodes.is_empty()
            )
        })
    }

    fn deep_clone_inner(&self, preserve_mounts: bool) -> Self {
        Self {
            vnode: Rc::new(VNodeInner {
                key: self.vnode.key.clone(),
                template: self.vnode.template,
                dynamic_values: self
                    .vnode
                    .dynamic_values
                    .iter()
                    .map(|value| match value {
                        DynamicValue::Node(DynamicNode::Fragment(nodes)) => {
                            DynamicValue::Node(DynamicNode::Fragment(
                                nodes
                                    .iter()
                                    .map(|node| {
                                        if preserve_mounts {
                                            node.clone_for_mount_snapshot()
                                        } else {
                                            node.deep_clone_inner(false)
                                        }
                                    })
                                    .collect(),
                            ))
                        }
                        DynamicValue::Node(other) => DynamicValue::Node(other.clone()),
                        DynamicValue::Attrs(attrs) => DynamicValue::Attrs(attrs.clone()),
                    })
                    .collect(),
            }),
            mount: Cell::new(if preserve_mounts {
                self.mount.get()
            } else {
                Self::UNMOUNTED_MOUNT
            }),
        }
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

/// A runtime value for one flat template dynamic slot.
#[derive(Debug, Clone)]
pub enum DynamicValue {
    /// A dynamic node value.
    Node(DynamicNode),
    /// A dynamic attribute list value.
    Attrs(Box<[Attribute]>),
}

impl DynamicValue {
    /// Return this value as a dynamic node if it is one.
    pub fn as_node(&self) -> Option<&DynamicNode> {
        match self {
            Self::Node(node) => Some(node),
            Self::Attrs(_) => None,
        }
    }

    /// Return this value as dynamic attributes if it is an attribute slot.
    pub fn as_attrs(&self) -> Option<&[Attribute]> {
        match self {
            Self::Attrs(attrs) => Some(attrs),
            Self::Node(_) => None,
        }
    }

    pub(crate) fn node(&self) -> &DynamicNode {
        self.as_node().expect("dynamic slot should contain a node")
    }

    pub(crate) fn attrs(&self) -> &[Attribute] {
        self.as_attrs()
            .expect("dynamic slot should contain attributes")
    }
}

/// An instance of a child component
pub struct VComponent {
    /// The name of this component
    pub name: &'static str,

    /// The rendering lifecycle for this component's scope, owning the props
    /// it renders from. Plain components use a body-running driver; portal
    /// and suspense attach drivers in `into_vcomponent` that manage the
    /// scope's output directly. The driver also identifies the component
    /// during diffing (see `RenderDriver::same_component`).
    pub(crate) driver: Rc<dyn crate::render_driver::RenderDriver>,
}

impl Clone for VComponent {
    fn clone(&self) -> Self {
        Self {
            name: self.name,
            driver: self.driver.duplicate(),
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
        Self::new_with_driver(
            fn_name,
            Rc::new(crate::render_driver::BodyDriver::new(
                component,
                <P as Properties>::memoize,
                props,
                fn_name,
            )),
        )
    }

    /// Create a new [`VComponent`] whose scope is rendered by `driver` from
    /// the props it owns.
    pub(crate) fn new_with_driver(
        fn_name: &'static str,
        driver: Rc<dyn crate::render_driver::RenderDriver>,
    ) -> Self {
        VComponent {
            name: fn_name,
            driver,
        }
    }

    /// Get the [`ScopeId`] this node is mounted to if it's mounted
    ///
    /// This is useful for rendering nodes outside of the VirtualDom, such as in SSR
    ///
    /// Returns [`None`] if the node is not mounted
    pub fn mounted_scope_id(
        &self,
        dynamic_node_index: usize,
        vnode: &VNode,
        dom: &VirtualDom,
    ) -> Option<ScopeId> {
        let mount = vnode.mounted_id()?;

        dom.mounted_dynamic_component_scope(mount, dynamic_node_index)
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
        let mount = vnode.mounted_id()?;

        let scope_id = dom.mounted_dynamic_component_scope(mount, dynamic_node_index)?;

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
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
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

pub trait IntoVNode {
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
