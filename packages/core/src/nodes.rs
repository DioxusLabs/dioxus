use crate::{
    Element, Event, Properties, ScopeId, VirtualDom,
    arena::ElementId,
    events::ListenerCallback,
    innerlude::{MountId, ScopeState},
    properties::ComponentFunction,
};
use const_vec::ConstVec;
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
            &[],
            &[TemplatePath::root(0)],
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
            &[],
            &[TemplatePath::root(0)],
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

    /// Get the mounted id for a dynamic text node index.
    ///
    /// Panics if this vnode or dynamic text slot is not mounted.
    pub fn unchecked_mounted_dynamic_node(
        &self,
        dynamic_node_idx: usize,
        dom: &VirtualDom,
    ) -> ElementId {
        self.mounted_dynamic_node(dynamic_node_idx, dom)
            .expect("dynamic text node slot should be mounted")
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

    /// Get the mounted id for a root node index.
    ///
    /// Panics if this vnode or root slot is not mounted.
    pub fn unchecked_mounted_root(&self, root_idx: usize, dom: &VirtualDom) -> ElementId {
        self.mounted_root(root_idx, dom)
            .expect("root node slot should be mounted")
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

    /// Get the mounted id for a dynamic attribute index.
    ///
    /// Panics if this vnode or dynamic attribute slot is not mounted.
    pub fn unchecked_mounted_dynamic_attribute(
        &self,
        dynamic_attribute_idx: usize,
        dom: &VirtualDom,
    ) -> ElementId {
        self.mounted_dynamic_attribute(dynamic_attribute_idx, dom)
            .expect("dynamic attribute slot should be mounted")
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
                                    .map(|node| node.deep_clone_inner(preserve_mounts))
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

type StaticTemplateOpArray = &'static [TemplateOp];
type StaticTemplateStringArray = &'static [&'static str];
type StaticTemplatePathArray = &'static [TemplatePath];

/// A compact path from a template root to a dynamic node or dynamic attribute.
///
/// Paths use the template-v2 child/sibling bit encoding: `1` means descend to the first child and
/// `0` means advance to the next sibling. Bits are appended by shifting left, so iteration decodes
/// from the least-significant bit back toward the root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TemplatePath {
    path: u128,
}

/// A single step in a compact template path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplatePathStep {
    /// Descend to the first child.
    Child,
    /// Advance to the next sibling.
    Sibling,
}

impl TemplatePath {
    /// Create an empty path.
    pub const fn empty() -> Self {
        Self { path: 0 }
    }

    /// Create a path from raw template-v2 bits.
    pub const fn from_bits(path: u128) -> Self {
        Self { path }
    }

    /// Return the path for a root position.
    pub const fn root(index: usize) -> Self {
        let mut path = Self::empty().next_child();
        let mut sibling = 0;
        while sibling < index {
            path = path.next_sibling();
            sibling += 1;
        }
        path
    }

    /// Return the raw template-v2 bits for this path.
    pub const fn bits(self) -> u128 {
        self.path
    }

    /// Return the path for the first child of this path.
    pub const fn next_child(self) -> Self {
        Self {
            path: (self.path << 1) | 1,
        }
    }

    /// Return the path for the next sibling of this path.
    pub const fn next_sibling(self) -> Self {
        Self {
            path: self.path << 1,
        }
    }

    /// Return the parent path.
    pub const fn parent(self) -> Self {
        Self {
            path: self.path >> 1,
        }
    }

    /// Return the number of path segments.
    pub fn len(self) -> usize {
        let mut count = 0;
        let mut path = self.path;
        while path != 0 {
            if path & 1 == 1 {
                count += 1;
            }
            path >>= 1;
        }
        count
    }

    /// Return true if this path has no segments.
    pub const fn is_empty(self) -> bool {
        self.path == 0
    }

    /// Return the path segment at `index`.
    pub fn segment(self, index: usize) -> u8 {
        let mut current_segment = 0usize;
        let mut current_index = 0u8;
        let mut started = false;
        for step in self.iter() {
            match step {
                TemplatePathStep::Child => {
                    if started {
                        if current_segment == index {
                            return current_index;
                        }
                        current_segment += 1;
                        current_index = 0;
                    } else {
                        started = true;
                    }
                }
                TemplatePathStep::Sibling => {
                    current_index = current_index
                        .checked_add(1)
                        .expect("template path sibling index overflow");
                }
            }
        }
        if started && current_segment == index {
            return current_index;
        }
        panic!("template path segment index out of bounds");
    }

    /// Return true if this path points at a root-level dynamic slot.
    pub fn is_root_level_slot(self) -> bool {
        self.len() == 1
    }

    /// Return true if this path starts with `root_idx`.
    pub fn starts_with_root(self, root_idx: u8) -> bool {
        !self.is_empty() && self.segment(0) == root_idx
    }

    /// Return true if this path is exactly a root-level slot.
    pub fn is_root_slot(self, root_idx: usize) -> bool {
        self.len() == 1 && self.segment(0) == root_idx as u8
    }

    /// Split this dynamic slot path into `(parent_path, insertion_index)`.
    pub fn split_slot(self) -> (TemplatePath, usize) {
        let mut parent = self.path;
        let mut insertion_index = 0usize;
        while parent != 0 && parent & 1 == 0 {
            insertion_index += 1;
            parent >>= 1;
        }
        if parent != 0 {
            parent >>= 1;
        }
        (TemplatePath { path: parent }, insertion_index)
    }

    /// Return the parent path of this dynamic slot.
    pub fn slot_parent(self) -> TemplatePath {
        self.split_slot().0
    }

    /// Return true if this static path is equal to or beneath `ancestor`.
    pub fn is_descendant_of_static(self, ancestor: TemplatePath) -> bool {
        self.starts_with(ancestor)
    }

    /// Return true if this dynamic slot is mounted inside `ancestor`.
    pub fn slot_is_inside_static(self, ancestor: TemplatePath) -> bool {
        self.slot_parent().starts_with(ancestor)
    }

    /// Return true if this compact path starts with `ancestor`.
    pub fn starts_with(self, ancestor: TemplatePath) -> bool {
        let self_len = self.bit_len();
        let ancestor_len = ancestor.bit_len();
        ancestor.path == 0
            || (ancestor_len <= self_len
                && (self.path >> (self_len - ancestor_len)) == ancestor.path)
    }

    /// Return the number of raw child/sibling bits in this path.
    pub fn bit_len(self) -> u32 {
        u128::BITS - self.path.leading_zeros()
    }

    /// Iterate over child/sibling path steps from root to leaf.
    pub fn iter(self) -> TemplatePathIter {
        TemplatePathIter {
            path: self.path,
            next_bit: self.bit_len(),
        }
    }
}

/// Iterator over compact template path steps.
pub struct TemplatePathIter {
    path: u128,
    next_bit: u32,
}

impl Iterator for TemplatePathIter {
    type Item = TemplatePathStep;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_bit == 0 {
            return None;
        }
        self.next_bit -= 1;
        let bit = (self.path >> self.next_bit) & 1;
        Some(if bit == 1 {
            TemplatePathStep::Child
        } else {
            TemplatePathStep::Sibling
        })
    }
}

#[cfg(feature = "serialize")]
impl serde::Serialize for TemplatePath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde::Serialize::serialize(&self.path, serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for TemplatePath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let deserialized = <u128 as serde::Deserialize>::deserialize(deserializer)?;
        Ok(Self::from_bits(deserialized))
    }
}

const fn static_str_eq(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left.len() != right.len() {
        return false;
    }

    let mut index = 0;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    true
}

/// Static attribute namespace information in a raw template tape.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TemplateRawAttrNamespace {
    /// No namespace.
    None,
    /// The built-in Dioxus style namespace.
    Style,
    /// A custom namespace.
    Custom(&'static str),
}

impl TemplateRawAttrNamespace {
    /// Create a raw namespace from the optional namespace used by `dioxus_elements`.
    pub const fn new(namespace: Option<&'static str>) -> Self {
        match namespace {
            Some(namespace) if static_str_eq(namespace, "style") => Self::Style,
            Some(namespace) => Self::Custom(namespace),
            None => Self::None,
        }
    }
}

/// One unlowered operation in a template tape.
///
/// The RSX macro emits this raw tape directly. [`TemplateStorage::build`] lowers it into packed
/// [`TemplateOp`]s and dynamic [`TemplatePath`]s in const context.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TemplateRawOp {
    /// Open an element.
    OpenElement {
        /// Static tag name.
        tag: &'static str,
        /// Optional element namespace.
        namespace: Option<&'static str>,
    },
    /// Close the current element.
    CloseElement,
    /// Static attribute on the current element.
    StaticAttr {
        /// Static attribute name.
        name: &'static str,
        /// Static attribute value.
        value: &'static str,
        /// Attribute namespace.
        namespace: TemplateRawAttrNamespace,
    },
    /// Dynamic attribute slot on the current element.
    DynamicAttr,
    /// Static text node.
    StaticText {
        /// Static text value.
        value: &'static str,
    },
    /// Dynamic node slot.
    DynamicNode,
}

impl TemplateRawOp {
    /// Create an open-element raw op.
    pub const fn open_element(tag: &'static str, namespace: Option<&'static str>) -> Self {
        Self::OpenElement { tag, namespace }
    }

    /// Create a close-element raw op.
    pub const fn close_element() -> Self {
        Self::CloseElement
    }

    /// Create a static-attribute raw op.
    pub const fn static_attr(
        name: &'static str,
        value: &'static str,
        namespace: Option<&'static str>,
    ) -> Self {
        Self::StaticAttr {
            name,
            value,
            namespace: TemplateRawAttrNamespace::new(namespace),
        }
    }

    /// Create a style static-attribute raw op.
    pub const fn style_attr(name: &'static str, value: &'static str) -> Self {
        Self::StaticAttr {
            name,
            value,
            namespace: TemplateRawAttrNamespace::Style,
        }
    }

    /// Create a dynamic-attribute raw op.
    pub const fn dynamic_attr() -> Self {
        Self::DynamicAttr
    }

    /// Create a static-text raw op.
    pub const fn static_text(value: &'static str) -> Self {
        Self::StaticText { value }
    }

    /// Create a dynamic-node raw op.
    pub const fn dynamic_node() -> Self {
        Self::DynamicNode
    }
}

/// One operation in a flat static template tape.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct TemplateOp(u16);

/// Decoded static attribute namespace storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodedTemplateAttrNamespace {
    /// No namespace.
    None,
    /// The built-in Dioxus style namespace.
    Style,
    /// A custom namespace string follows the static attr name/value.
    Custom,
}

/// Decoded representation of a packed [`TemplateOp`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodedTemplateOp {
    /// Enter an element. `skip` is the number of ops in this element subtree.
    Enter {
        /// Number of ops to skip to move past this element and its children.
        skip: u16,
        /// Whether the reserved namespace string slot contains a namespace.
        namespace: bool,
    },
    /// A static attribute on the current element.
    Attr {
        /// Namespace storage for this attr.
        namespace: DecodedTemplateAttrNamespace,
    },
    /// A text node marker. The next op is either [`Self::Static`] or [`Self::Dynamic`].
    Text,
    /// A static string pool reference.
    Static(u16),
    /// A dynamic slot.
    Dynamic,
}

impl TemplateOp {
    const ENTER_MAX_CODE: u16 = 0x7fff;
    const ATTR_CODE: u16 = 0x8000;
    const ATTR_STYLE_CODE: u16 = 0x8001;
    const ATTR_CUSTOM_NS_CODE: u16 = 0x8002;
    const TEXT_CODE: u16 = 0x8003;
    const DYN_CODE: u16 = 0x8004;
    const STATIC_BASE: u16 = 0x8005;
    const MAX_CAP: usize = 16_383;

    /// Create a packed enter op.
    pub const fn enter(skip: u16, namespace: bool) -> Self {
        if skip as usize > Self::MAX_CAP {
            panic!("op skip exceeds packed op capacity");
        }
        Self((skip << 1) | namespace as u16)
    }

    /// Create a packed static attribute op.
    pub const fn attr(namespace: bool) -> Self {
        if namespace {
            Self(Self::ATTR_STYLE_CODE)
        } else {
            Self(Self::ATTR_CODE)
        }
    }

    /// Create a packed static attribute op with a following custom namespace string.
    pub const fn attr_custom_namespace() -> Self {
        Self(Self::ATTR_CUSTOM_NS_CODE)
    }

    /// Create a packed text marker op.
    pub const fn text() -> Self {
        Self(Self::TEXT_CODE)
    }

    /// Create a packed static string reference op.
    pub const fn static_text(id: u16) -> Self {
        if id as usize >= Self::MAX_CAP {
            panic!("static op id exceeds packed op capacity");
        }
        Self(Self::STATIC_BASE + id)
    }

    /// Create a packed dynamic op.
    pub const fn dynamic() -> Self {
        Self(Self::DYN_CODE)
    }

    /// Decode this packed op.
    pub const fn decode(self) -> DecodedTemplateOp {
        if self.0 <= Self::ENTER_MAX_CODE {
            DecodedTemplateOp::Enter {
                skip: self.0 >> 1,
                namespace: self.0 & 1 == 1,
            }
        } else if self.0 == Self::ATTR_CODE {
            DecodedTemplateOp::Attr {
                namespace: DecodedTemplateAttrNamespace::None,
            }
        } else if self.0 == Self::ATTR_STYLE_CODE {
            DecodedTemplateOp::Attr {
                namespace: DecodedTemplateAttrNamespace::Style,
            }
        } else if self.0 == Self::ATTR_CUSTOM_NS_CODE {
            DecodedTemplateOp::Attr {
                namespace: DecodedTemplateAttrNamespace::Custom,
            }
        } else if self.0 == Self::TEXT_CODE {
            DecodedTemplateOp::Text
        } else if self.0 == Self::DYN_CODE {
            DecodedTemplateOp::Dynamic
        } else {
            DecodedTemplateOp::Static(self.0 - Self::STATIC_BASE)
        }
    }

    /// Return true if this op enters an element.
    pub const fn is_enter(self) -> bool {
        matches!(self.decode(), DecodedTemplateOp::Enter { .. })
    }

    /// Return true if this op starts a static attr.
    pub const fn is_attr(self) -> bool {
        matches!(self.decode(), DecodedTemplateOp::Attr { .. })
    }

    /// Return true if this op starts a text slot.
    pub const fn is_text(self) -> bool {
        matches!(self.decode(), DecodedTemplateOp::Text)
    }

    /// Return true if this op is a dynamic slot.
    pub const fn is_dynamic(self) -> bool {
        matches!(self.decode(), DecodedTemplateOp::Dynamic)
    }

    /// Return the namespace bit for element and attr ops.
    pub const fn has_namespace(self) -> bool {
        match self.decode() {
            DecodedTemplateOp::Enter { namespace, .. } => namespace,
            DecodedTemplateOp::Attr { namespace } => {
                !matches!(namespace, DecodedTemplateAttrNamespace::None)
            }
            _ => false,
        }
    }

    /// Return the element skip encoded in an enter op.
    pub const fn enter_skip(self) -> Option<u16> {
        match self.decode() {
            DecodedTemplateOp::Enter { skip, .. } => Some(skip),
            _ => None,
        }
    }

    /// Return the static string id encoded in a static op.
    pub const fn static_id(self) -> Option<u16> {
        match self.decode() {
            DecodedTemplateOp::Static(id) => Some(id),
            _ => None,
        }
    }
}

/// Maximum packed template storage capacity.
#[doc(hidden)]
pub const TEMPLATE_STORAGE_MAX_CAP: usize = TemplateOp::MAX_CAP;

const TEMPLATE_PATH_STACK_CAP: usize = 129;

/// Const storage for a lowered raw template.
///
/// The RSX macro emits a `static TemplateStorage<N>` from a raw operation tape, then calls
/// [`Self::as_template`] to expose the compact [`Template`] used by the runtime.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct TemplateStorage<const CAP: usize> {
    ops: ConstVec<TemplateOp, CAP>,
    strings: ConstVec<&'static str, CAP>,
    dynamics: ConstVec<TemplatePath, CAP>,
}

impl<const CAP: usize> TemplateStorage<CAP> {
    /// Lower a raw template tape into packed storage in const context.
    pub const fn build(raw: &'static [TemplateRawOp]) -> Self {
        let mut storage = Self {
            ops: ConstVec::new_with_max_size(),
            strings: ConstVec::new_with_max_size(),
            dynamics: ConstVec::new_with_max_size(),
        };

        let mut enter_stack = [0usize; TEMPLATE_PATH_STACK_CAP];
        let mut element_paths = [TemplatePath::empty(); TEMPLATE_PATH_STACK_CAP];
        let mut next_paths = [TemplatePath::empty(); TEMPLATE_PATH_STACK_CAP];
        next_paths[0] = TemplatePath::root(0);
        let mut stack_pointer = 0usize;

        let mut index = 0usize;
        while index < raw.len() {
            match raw[index] {
                TemplateRawOp::OpenElement { tag, namespace } => {
                    if stack_pointer + 1 >= TEMPLATE_PATH_STACK_CAP {
                        panic!("template path stack capacity exceeded");
                    }

                    let path = next_paths[stack_pointer];
                    next_paths[stack_pointer] = path.next_sibling();
                    element_paths[stack_pointer] = path;
                    enter_stack[stack_pointer] = storage.ops.len();
                    next_paths[stack_pointer + 1] = path.next_child();
                    stack_pointer += 1;

                    storage.ops = storage.ops.push(TemplateOp::enter(0, namespace.is_some()));
                    storage.push_static(tag);
                    if let Some(namespace) = namespace {
                        storage.push_static(namespace);
                    }
                }
                TemplateRawOp::CloseElement => {
                    if stack_pointer == 0 {
                        panic!("template close op without matching open op");
                    }

                    stack_pointer -= 1;
                    let enter_index = enter_stack[stack_pointer];
                    let namespace = storage.ops.at(enter_index).has_namespace();
                    let skip = storage.ops.len() - enter_index;
                    if skip > TemplateOp::MAX_CAP {
                        panic!("template op skip exceeds packed op capacity");
                    }
                    storage.ops = storage
                        .ops
                        .set(enter_index, TemplateOp::enter(skip as u16, namespace));
                }
                TemplateRawOp::StaticAttr {
                    name,
                    value,
                    namespace,
                } => {
                    match namespace {
                        TemplateRawAttrNamespace::None => {
                            storage.ops = storage.ops.push(TemplateOp::attr(false))
                        }
                        TemplateRawAttrNamespace::Style => {
                            storage.ops = storage.ops.push(TemplateOp::attr(true))
                        }
                        TemplateRawAttrNamespace::Custom(_) => {
                            storage.ops = storage.ops.push(TemplateOp::attr_custom_namespace())
                        }
                    }
                    storage.push_static(name);
                    storage.push_static(value);
                    if let TemplateRawAttrNamespace::Custom(namespace) = namespace {
                        storage.push_static(namespace);
                    }
                }
                TemplateRawOp::DynamicAttr => {
                    if stack_pointer == 0 {
                        panic!("dynamic attr raw op without an open element");
                    }
                    storage.ops = storage.ops.push(TemplateOp::dynamic());
                    storage.dynamics = storage.dynamics.push(element_paths[stack_pointer - 1]);
                }
                TemplateRawOp::StaticText { value } => {
                    let path = next_paths[stack_pointer];
                    next_paths[stack_pointer] = path.next_sibling();
                    storage.ops = storage.ops.push(TemplateOp::text());
                    storage.push_static(value);
                }
                TemplateRawOp::DynamicNode => {
                    let path = next_paths[stack_pointer];
                    next_paths[stack_pointer] = path.next_sibling();
                    storage.ops = storage.ops.push(TemplateOp::text());
                    storage.ops = storage.ops.push(TemplateOp::dynamic());
                    storage.dynamics = storage.dynamics.push(path);
                }
            }
            index += 1;
        }

        if stack_pointer != 0 {
            panic!("template raw ops ended with unclosed elements");
        }

        storage
    }

    /// Return this storage as a compact template.
    pub const fn as_template(&'static self) -> Template {
        Template::new(
            self.ops.as_slice(),
            self.strings.as_slice(),
            self.dynamics.as_slice(),
        )
    }

    const fn push_static(&mut self, value: &'static str) {
        let mut id = 0usize;
        while id < self.strings.len() {
            if static_str_eq(self.strings.at(id), value) {
                self.ops = self.ops.push(TemplateOp::static_text(id as u16));
                return;
            }
            id += 1;
        }

        if self.strings.len() > u16::MAX as usize {
            panic!("template string capacity exceeded");
        }
        let id = self.strings.len();
        self.strings = self.strings.push(value);
        self.ops = self.ops.push(TemplateOp::static_text(id as u16));
    }
}

impl Template {
    /// Lower a raw template tape into a leaked runtime template.
    ///
    /// This mirrors [`TemplateStorage::build`] without allocating the max-capacity
    /// const storage on the runtime stack.
    #[doc(hidden)]
    pub fn from_raw_ops(raw: &'static [TemplateRawOp]) -> Self {
        let mut ops = Vec::new();
        let mut strings = Vec::new();
        let mut dynamics = Vec::new();

        let mut enter_stack = [0usize; TEMPLATE_PATH_STACK_CAP];
        let mut element_paths = [TemplatePath::empty(); TEMPLATE_PATH_STACK_CAP];
        let mut next_paths = [TemplatePath::empty(); TEMPLATE_PATH_STACK_CAP];
        next_paths[0] = TemplatePath::root(0);
        let mut stack_pointer = 0usize;

        for raw_op in raw {
            match *raw_op {
                TemplateRawOp::OpenElement { tag, namespace } => {
                    if stack_pointer + 1 >= TEMPLATE_PATH_STACK_CAP {
                        panic!("template path stack capacity exceeded");
                    }

                    let path = next_paths[stack_pointer];
                    next_paths[stack_pointer] = path.next_sibling();
                    element_paths[stack_pointer] = path;
                    enter_stack[stack_pointer] = ops.len();
                    next_paths[stack_pointer + 1] = path.next_child();
                    stack_pointer += 1;

                    ops.push(TemplateOp::enter(0, namespace.is_some()));
                    push_runtime_static(&mut ops, &mut strings, tag);
                    if let Some(namespace) = namespace {
                        push_runtime_static(&mut ops, &mut strings, namespace);
                    }
                }
                TemplateRawOp::CloseElement => {
                    if stack_pointer == 0 {
                        panic!("template close op without matching open op");
                    }

                    stack_pointer -= 1;
                    let enter_index = enter_stack[stack_pointer];
                    let namespace = ops[enter_index].has_namespace();
                    let skip = ops.len() - enter_index;
                    if skip > TemplateOp::MAX_CAP {
                        panic!("template op skip exceeds packed op capacity");
                    }
                    ops[enter_index] = TemplateOp::enter(skip as u16, namespace);
                }
                TemplateRawOp::StaticAttr {
                    name,
                    value,
                    namespace,
                } => {
                    match namespace {
                        TemplateRawAttrNamespace::None => ops.push(TemplateOp::attr(false)),
                        TemplateRawAttrNamespace::Style => ops.push(TemplateOp::attr(true)),
                        TemplateRawAttrNamespace::Custom(_) => {
                            ops.push(TemplateOp::attr_custom_namespace())
                        }
                    }
                    push_runtime_static(&mut ops, &mut strings, name);
                    push_runtime_static(&mut ops, &mut strings, value);
                    if let TemplateRawAttrNamespace::Custom(namespace) = namespace {
                        push_runtime_static(&mut ops, &mut strings, namespace);
                    }
                }
                TemplateRawOp::DynamicAttr => {
                    if stack_pointer == 0 {
                        panic!("dynamic attr raw op without an open element");
                    }
                    ops.push(TemplateOp::dynamic());
                    dynamics.push(element_paths[stack_pointer - 1]);
                }
                TemplateRawOp::StaticText { value } => {
                    let path = next_paths[stack_pointer];
                    next_paths[stack_pointer] = path.next_sibling();
                    ops.push(TemplateOp::text());
                    push_runtime_static(&mut ops, &mut strings, value);
                }
                TemplateRawOp::DynamicNode => {
                    let path = next_paths[stack_pointer];
                    next_paths[stack_pointer] = path.next_sibling();
                    ops.push(TemplateOp::text());
                    ops.push(TemplateOp::dynamic());
                    dynamics.push(path);
                }
            }
        }

        if stack_pointer != 0 {
            panic!("template raw ops ended with unclosed elements");
        }

        Self::new(
            Box::leak(ops.into_boxed_slice()),
            Box::leak(strings.into_boxed_slice()),
            Box::leak(dynamics.into_boxed_slice()),
        )
    }
}

fn push_runtime_static(
    ops: &mut Vec<TemplateOp>,
    strings: &mut Vec<&'static str>,
    value: &'static str,
) {
    for (id, current) in strings.iter().enumerate() {
        if *current == value {
            ops.push(TemplateOp::static_text(id as u16));
            return;
        }
    }

    if strings.len() > u16::MAX as usize {
        panic!("template string capacity exceeded");
    }
    let id = strings.len();
    strings.push(value);
    ops.push(TemplateOp::static_text(id as u16));
}

impl std::fmt::Debug for TemplateOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.decode().fmt(f)
    }
}

/// A static layout of a UI tree that describes a set of dynamic and static nodes.
///
/// This is the core innovation in Dioxus. Most UIs are made of static nodes, yet participate in diffing like any
/// dynamic node. This struct can be created at compile time. It promises that its pointer is unique, allow Dioxus to use
/// its static description of the UI to skip immediately to the dynamic nodes during diffing.
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, Eq, PartialOrd, Ord)]
pub struct Template {
    /// Flat static template operations.
    #[cfg_attr(feature = "serialize", serde(deserialize_with = "deserialize_leaky"))]
    ops: StaticTemplateOpArray,

    /// Static strings referenced by [`TemplateOp::Static`].
    #[cfg_attr(
        feature = "serialize",
        serde(deserialize_with = "deserialize_string_array_leaky")
    )]
    strings: StaticTemplateStringArray,

    /// Dynamic paths in document order.
    #[cfg_attr(feature = "serialize", serde(deserialize_with = "deserialize_leaky"))]
    dynamics: StaticTemplatePathArray,

    /// Compile-time hash of template content for reliable cross-crate comparison.
    /// This ensures identical templates compare equal regardless of optimization levels.
    ///
    /// Uses xxh64 (64-bit hash). By the birthday paradox, collision probability is:
    /// P ≈ 1 - e^(-n²/(2 × 2^64)) where n = number of templates.
    ///
    /// - 1,000 templates: P ≈ 2.7 × 10^-14 (essentially zero)
    /// - 10,000 templates: P ≈ 2.7 × 10^-12 (essentially zero)
    /// - 1 million templates: P ≈ 0.000003%
    /// - 50% collision chance requires ~5 billion templates
    ///
    /// For any realistic application, collision probability is negligible.
    hash: u64,
}

impl Template {
    /// Create a new flat template with the given ops, strings, and dynamic paths.
    /// The hash is computed automatically from the template content.
    pub const fn new(
        ops: &'static [TemplateOp],
        strings: &'static [&'static str],
        dynamics: &'static [TemplatePath],
    ) -> Self {
        Self {
            ops,
            strings,
            dynamics,
            hash: Self::compute_hash(ops, strings, dynamics),
        }
    }

    /// Get the flat template operations.
    pub const fn ops(&self) -> &'static [TemplateOp] {
        self.ops
    }

    /// Get the template static string pool.
    pub const fn strings(&self) -> &'static [&'static str] {
        self.strings
    }

    /// Get dynamic paths in document order.
    pub const fn dynamics(&self) -> &'static [TemplatePath] {
        self.dynamics
    }

    /// Get the number of root positions in this template.
    pub fn root_count(&self) -> usize {
        let mut count = 0;
        let mut op = 0;
        while op < self.ops.len() {
            if self.is_static_node_op(op) || self.is_dynamic_node_marker(op) {
                count += 1;
            }
            op = self.next_sibling_op(op);
        }
        count
    }

    /// Get a static string from this template's string pool.
    pub const fn string(&self, id: u16) -> &'static str {
        self.strings[id as usize]
    }

    /// Decode an element op into its subtree length and namespace presence.
    pub fn enter_meta(&self, op: usize) -> Option<(usize, bool)> {
        match self.ops.get(op).map(|op| op.decode()) {
            Some(DecodedTemplateOp::Enter { skip, namespace }) => Some((skip as usize, namespace)),
            _ => None,
        }
    }

    /// Return the static string referenced by an op.
    pub fn static_string_at_op(&self, op: usize) -> Option<&'static str> {
        match self.ops.get(op).map(|op| op.decode()) {
            Some(DecodedTemplateOp::Static(id)) => Some(self.string(id)),
            _ => None,
        }
    }

    /// Return the tag and namespace for an element op.
    pub fn element_meta_at_op(&self, op: usize) -> Option<(&'static str, Option<&'static str>)> {
        let (_, has_namespace) = self.enter_meta(op)?;
        let tag = self.static_string_at_op(op + 1)?;
        let namespace = has_namespace
            .then(|| self.static_string_at_op(op + 2))
            .flatten();
        Some((tag, namespace))
    }

    /// Return the first child/attribute op inside an element.
    pub fn element_children_start(&self, op: usize) -> Option<usize> {
        let (_, has_namespace) = self.enter_meta(op)?;
        Some(op + if has_namespace { 3 } else { 2 })
    }

    /// Return the name, value, and namespace for a static attr op.
    pub fn static_attr_at_op(
        &self,
        op: usize,
    ) -> Option<(&'static str, &'static str, Option<&'static str>)> {
        let namespace = match self.ops.get(op).map(|op| op.decode()) {
            Some(DecodedTemplateOp::Attr { namespace }) => namespace,
            _ => return None,
        };
        let name = self.static_string_at_op(op + 1)?;
        let value = self.static_string_at_op(op + 2)?;
        let namespace = match namespace {
            DecodedTemplateAttrNamespace::None => None,
            DecodedTemplateAttrNamespace::Style => Some("style"),
            DecodedTemplateAttrNamespace::Custom => self.static_string_at_op(op + 3),
        };
        Some((name, value, namespace))
    }

    /// Return the text for a static `Text, Static` node marker.
    pub fn static_text_at_op(&self, op: usize) -> Option<&'static str> {
        (self.ops.get(op).map(|op| op.decode()) == Some(DecodedTemplateOp::Text))
            .then(|| self.static_string_at_op(op + 1))
            .flatten()
    }

    /// Return the number of ops used by a static attr at `op`.
    pub fn attr_op_len(&self, op: usize) -> Option<usize> {
        match self.ops.get(op).map(|op| op.decode()) {
            Some(DecodedTemplateOp::Attr {
                namespace: DecodedTemplateAttrNamespace::Custom,
            }) => Some(4),
            Some(DecodedTemplateOp::Attr { .. }) => Some(3),
            _ => None,
        }
    }

    /// Return the op immediately after an element subtree.
    pub fn element_end(&self, op: usize) -> Option<usize> {
        let (skip, _) = self.enter_meta(op)?;
        Some(op + skip)
    }

    /// Return the first static or dynamic child marker inside an element.
    pub fn first_child_node_op(&self, element_op: usize) -> Option<usize> {
        let mut cursor = self.element_children_start(element_op)?;
        let end = self.element_end(element_op)?;
        while cursor < end {
            if let Some(len) = self.attr_op_len(cursor) {
                cursor += len;
            } else if self.dynamic_op_is_attr(cursor) {
                cursor += 1;
            } else {
                break;
            }
        }
        Some(cursor)
    }

    /// Find a static attr fallback value for a key in an element.
    pub fn static_attr_value_for_key(
        &self,
        element_op: usize,
        key: (&'static str, Option<&'static str>),
    ) -> Option<&'static str> {
        let mut cursor = self.element_children_start(element_op)?;
        let end = self.first_child_node_op(element_op)?;
        let mut found = None;
        while cursor < end {
            if let Some((name, value, namespace)) = self.static_attr_at_op(cursor) {
                if (name, namespace) == key {
                    found = Some(value);
                }
                cursor += self.attr_op_len(cursor)?;
            } else if self.dynamic_op_is_attr(cursor) {
                cursor += 1;
            } else {
                break;
            }
        }
        found
    }

    /// Iterate over dynamic node slots as `(dynamic_value_index, path)` pairs.
    pub fn node_paths(&self) -> impl Iterator<Item = (usize, TemplatePath)> + '_ {
        self.dynamics
            .iter()
            .copied()
            .enumerate()
            .filter(move |(idx, _)| self.dynamic_is_node(*idx))
    }

    /// Iterate over dynamic attribute slots as `(dynamic_value_index, path)` pairs.
    pub fn attr_paths(&self) -> impl Iterator<Item = (usize, TemplatePath)> + '_ {
        self.dynamics
            .iter()
            .copied()
            .enumerate()
            .filter(move |(idx, _)| self.dynamic_is_attr(*idx))
    }

    /// Get the template path for a dynamic slot by dynamic value index.
    pub fn dynamic_path(&self, idx: usize) -> TemplatePath {
        self.dynamics[idx]
    }

    /// Return the flat op index for a static root at a root position.
    pub fn root_op_index(&self, root_idx: usize) -> Option<usize> {
        let mut current_root = 0;
        let mut idx = 0;
        while idx < self.ops.len() {
            if self.is_static_node_op(idx) || self.is_dynamic_node_marker(idx) {
                if current_root == root_idx {
                    return self.is_static_node_op(idx).then_some(idx);
                }
                current_root += 1;
                idx = self.next_sibling_op(idx);
            } else {
                idx = self.next_sibling_op(idx);
            }
        }
        None
    }

    /// Return the flat op index for a static node path.
    pub fn static_node_op_at_path(&self, path: TemplatePath) -> Option<usize> {
        if path.is_empty() {
            return None;
        }
        let mut op = self.root_op_index(path.segment(0) as usize)?;
        for depth in 1..path.len() {
            op = self.static_child_op(op, path.segment(depth) as usize)?;
        }
        Some(op)
    }

    /// Return child indexes for navigating from a static root through a static prototype.
    pub fn static_prototype_child_indexes(&self, path: TemplatePath) -> Option<Vec<usize>> {
        if path.is_empty() {
            return None;
        }
        let mut op = self.root_op_index(path.segment(0) as usize)?;
        let mut indexes = Vec::new();
        for depth in 1..path.len() {
            let (child_op, prototype_index) =
                self.static_child_op_and_prototype_index(op, path.segment(depth) as usize)?;
            indexes.push(prototype_index);
            op = child_op;
        }
        Some(indexes)
    }

    /// Return the static-prototype insertion index for a dynamic child slot.
    pub fn static_prototype_insertion_index(
        &self,
        parent_path: TemplatePath,
        child_idx: usize,
    ) -> Option<usize> {
        let parent_op = self.static_node_op_at_path(parent_path)?;
        self.static_child_insertion_index(parent_op, child_idx)
    }

    /// Return the flat op index for the static child at `child_idx` under an element op.
    pub fn static_child_op(&self, element_op: usize, child_idx: usize) -> Option<usize> {
        self.static_child_op_and_prototype_index(element_op, child_idx)
            .map(|(op, _)| op)
    }

    /// Return the static-prototype child index where authored child `child_idx` should be inserted.
    pub fn static_child_insertion_index(
        &self,
        element_op: usize,
        child_idx: usize,
    ) -> Option<usize> {
        let (skip, _) = self.enter_meta(element_op)?;
        let mut cursor = self.first_child_node_op(element_op)?;
        let end = element_op + skip;
        let mut child = 0;
        let mut static_child = 0;

        while cursor < end {
            if self.is_static_node_op(cursor) || self.is_dynamic_node_marker(cursor) {
                if child == child_idx {
                    return Some(static_child);
                }
                if self.is_static_node_op(cursor) {
                    static_child += 1;
                }
                child += 1;
                cursor = self.next_sibling_op(cursor);
            } else {
                cursor += 1;
            }
        }

        (child_idx >= child).then_some(static_child)
    }

    fn static_child_op_and_prototype_index(
        &self,
        element_op: usize,
        child_idx: usize,
    ) -> Option<(usize, usize)> {
        let (skip, _) = self.enter_meta(element_op)?;
        let mut cursor = self.element_children_start(element_op)?;
        let end = element_op + skip;

        while cursor < end {
            if let Some(len) = self.attr_op_len(cursor) {
                cursor += len;
            } else if self.dynamic_op_is_attr(cursor) {
                cursor += 1;
            } else if self.is_static_node_op(cursor) || self.is_dynamic_node_marker(cursor) {
                break;
            } else {
                return None;
            }
        }

        let mut child = 0;
        let mut static_child = 0;
        while cursor < end {
            if self.is_static_node_op(cursor) || self.is_dynamic_node_marker(cursor) {
                if child == child_idx {
                    return self
                        .is_static_node_op(cursor)
                        .then_some((cursor, static_child));
                }
                if self.is_static_node_op(cursor) {
                    static_child += 1;
                }
                child += 1;
                cursor = self.next_sibling_op(cursor);
            } else {
                cursor = self.next_sibling_op(cursor);
            }
        }

        None
    }

    /// Return the flat op index immediately after the static node or op at `op`.
    pub fn next_sibling_op(&self, op: usize) -> usize {
        match self.ops[op].decode() {
            DecodedTemplateOp::Enter { skip, .. } => op + skip as usize,
            DecodedTemplateOp::Text => op + 2,
            DecodedTemplateOp::Attr {
                namespace: DecodedTemplateAttrNamespace::Custom,
            } => op + 4,
            DecodedTemplateOp::Attr { .. } => op + 3,
            _ => op + 1,
        }
    }

    /// Return true if an op starts an element or static text node.
    pub fn is_static_node_op(&self, op: usize) -> bool {
        match self.ops[op].decode() {
            DecodedTemplateOp::Enter { .. } => true,
            DecodedTemplateOp::Text => matches!(
                self.ops.get(op + 1).map(|op| op.decode()),
                Some(DecodedTemplateOp::Static(_))
            ),
            _ => false,
        }
    }

    /// Return true if an op starts a dynamic node marker.
    pub fn is_dynamic_node_marker(&self, op: usize) -> bool {
        self.ops[op].decode() == DecodedTemplateOp::Text
            && matches!(
                self.ops.get(op + 1).map(|op| op.decode()),
                Some(DecodedTemplateOp::Dynamic)
            )
    }

    /// Return the packed op index for a dynamic slot.
    pub fn dynamic_op_index(&self, dynamic_idx: usize) -> Option<usize> {
        let mut seen = 0;
        for (idx, op) in self.ops.iter().enumerate() {
            if op.decode() == DecodedTemplateOp::Dynamic {
                if seen == dynamic_idx {
                    return Some(idx);
                }
                seen += 1;
            }
        }
        None
    }

    /// Return the dynamic value index for a packed dynamic op.
    pub fn dynamic_index_at_op(&self, op_index: usize) -> Option<usize> {
        if self.ops.get(op_index).map(|op| op.decode()) != Some(DecodedTemplateOp::Dynamic) {
            return None;
        }
        let mut seen = 0;
        for (idx, op) in self.ops.iter().enumerate() {
            if op.decode() == DecodedTemplateOp::Dynamic {
                if idx == op_index {
                    return Some(seen);
                }
                seen += 1;
            }
        }
        None
    }

    /// Return true if the dynamic op at `op_index` is a dynamic node slot.
    pub fn dynamic_op_is_node(&self, op_index: usize) -> bool {
        self.ops.get(op_index).is_some_and(|op| {
            op.decode() == DecodedTemplateOp::Dynamic
                && op_index > 0
                && self.ops[op_index - 1].decode() == DecodedTemplateOp::Text
        })
    }

    /// Return true if the dynamic op at `op_index` is a dynamic attribute slot.
    pub fn dynamic_op_is_attr(&self, op_index: usize) -> bool {
        self.ops.get(op_index).is_some_and(|op| {
            op.decode() == DecodedTemplateOp::Dynamic
                && (op_index == 0 || self.ops[op_index - 1].decode() != DecodedTemplateOp::Text)
        })
    }

    /// Return true if a dynamic slot is a node slot.
    pub fn dynamic_is_node(&self, dynamic_idx: usize) -> bool {
        self.dynamic_op_index(dynamic_idx)
            .is_some_and(|op| self.dynamic_op_is_node(op))
    }

    /// Return true if a dynamic slot is an attribute slot.
    pub fn dynamic_is_attr(&self, dynamic_idx: usize) -> bool {
        self.dynamic_op_index(dynamic_idx)
            .is_some_and(|op| self.dynamic_op_is_attr(op))
    }

    /// Compute a content-based hash of template structure.
    /// This is const so it can be used both at compile time and runtime.
    const fn compute_hash(
        ops: &[TemplateOp],
        strings: &[&'static str],
        dynamics: &[TemplatePath],
    ) -> u64 {
        use xxhash_rust::const_xxh64::xxh64;

        let mut hash = 0u64;

        let mut i = 0;
        while i < ops.len() {
            hash = match ops[i].decode() {
                DecodedTemplateOp::Enter { skip, namespace } => {
                    let mut h = xxh64(&[0x01], hash);
                    h = xxh64(&skip.to_le_bytes(), h);
                    xxh64(&[namespace as u8], h)
                }
                DecodedTemplateOp::Attr { namespace } => {
                    let h = xxh64(&[0x02], hash);
                    xxh64(&[namespace as u8], h)
                }
                DecodedTemplateOp::Text => xxh64(&[0x03], hash),
                DecodedTemplateOp::Static(id) => {
                    let h = xxh64(&[0x04], hash);
                    xxh64(strings[id as usize].as_bytes(), h)
                }
                DecodedTemplateOp::Dynamic => xxh64(&[0x05], hash),
            };
            i += 1;
        }

        // Hash dynamic metadata.
        hash = xxh64(&[0xA1], hash);
        let mut i = 0;
        while i < dynamics.len() {
            hash = xxh64(&dynamics[i].path.to_le_bytes(), hash);
            i += 1;
        }

        hash
    }
}

impl std::hash::Hash for Template {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

impl PartialEq for Template {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

#[cfg(feature = "serialize")]
pub(crate) fn deserialize_string_leaky<'a, 'de, D>(
    deserializer: D,
) -> Result<&'static str, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    let deserialized = String::deserialize(deserializer)?;
    Ok(&*Box::leak(deserialized.into_boxed_str()))
}

#[cfg(feature = "serialize")]
pub(crate) fn deserialize_string_array_leaky<'de, D>(
    deserializer: D,
) -> Result<&'static [&'static str], D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    let deserialized = Vec::<String>::deserialize(deserializer)?;
    let deserialized = deserialized
        .into_iter()
        .map(|value| &*Box::leak(value.into_boxed_str()) as &'static str)
        .collect::<Vec<_>>();
    Ok(&*Box::leak(deserialized.into_boxed_slice()))
}

#[cfg(feature = "serialize")]
pub(crate) fn deserialize_leaky<'a, 'de, T, D>(deserializer: D) -> Result<&'static [T], D::Error>
where
    T: serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    let deserialized = Box::<[T]>::deserialize(deserializer)?;
    Ok(&*Box::leak(deserialized))
}

#[cfg(feature = "serialize")]
pub(crate) fn deserialize_option_leaky<'a, 'de, D>(
    deserializer: D,
) -> Result<Option<&'static str>, D::Error>
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
        !self
            .ops
            .iter()
            .enumerate()
            .any(|(idx, _)| self.is_static_node_op(idx))
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

    /// Get the [`ScopeId`] this node is mounted to.
    ///
    /// Panics if the vnode or component slot is not mounted.
    pub fn unchecked_mounted_scope_id(
        &self,
        dynamic_node_index: usize,
        vnode: &VNode,
        dom: &VirtualDom,
    ) -> ScopeId {
        let mount = vnode.unchecked_mounted_id();

        dom.unchecked_mounted_dynamic_component_scope(mount, dynamic_node_index)
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

    /// Get the scope this node is mounted to.
    ///
    /// Panics if the vnode or component slot is not mounted.
    pub fn unchecked_mounted_scope<'a>(
        &self,
        dynamic_node_index: usize,
        vnode: &VNode,
        dom: &'a VirtualDom,
    ) -> &'a ScopeState {
        let scope_id = self.unchecked_mounted_scope_id(dynamic_node_index, vnode, dom);

        dom.scopes
            .get(scope_id.index())
            .expect("component scope should be live")
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
            Ok(val) => val.into_dyn_node(),
            _ => DynamicNode::default(),
        }
    }
}
impl IntoDynNode for Element {
    fn into_dyn_node(self) -> DynamicNode {
        match self {
            Ok(val) => val.into_dyn_node(),
            _ => DynamicNode::default(),
        }
    }
}
impl IntoDynNode for &Option<VNode> {
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
            Ok(val) => val.into_vnode(),
            _ => VNode::default(),
        }
    }
}
impl IntoVNode for &Element {
    fn into_vnode(self) -> VNode {
        match self {
            Ok(val) => val.into_vnode(),
            _ => VNode::default(),
        }
    }
}
impl IntoVNode for Option<VNode> {
    fn into_vnode(self) -> VNode {
        match self {
            Some(val) => val.into_vnode(),
            _ => VNode::default(),
        }
    }
}
impl IntoVNode for &Option<VNode> {
    fn into_vnode(self) -> VNode {
        match self.as_ref() {
            Some(val) => val.clone().into_vnode(),
            _ => VNode::default(),
        }
    }
}
impl IntoVNode for Option<Element> {
    fn into_vnode(self) -> VNode {
        match self {
            Some(val) => val.into_vnode(),
            _ => VNode::default(),
        }
    }
}
impl IntoVNode for &Option<Element> {
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
