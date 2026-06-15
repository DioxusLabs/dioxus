use crate::{
    Element, Event, Properties, ScopeId, VirtualDom,
    arena::ElementId,
    events::ListenerCallback,
    innerlude::{MountId, ScopeState},
    properties::ComponentFunction,
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

    /// The dynamic nodes in the template
    pub dynamic_nodes: Box<[DynamicNode]>,

    /// The dynamic attribute slots in the template
    ///
    /// This is a list of positions in the template where dynamic attributes can be inserted.
    ///
    /// The inner list *must* be in the format [static named attributes, remaining dynamically named attributes].
    /// More than one slot can point at the same template element when named dynamic attributes and
    /// spread attributes are mixed. Creation writes those slots in order, and diffing groups slots
    /// with the same attribute path so duplicate keys keep the same last-write-wins behavior and
    /// removed dynamic overrides can reveal the static template attribute underneath.
    ///
    /// For example:
    /// ```rust
    /// # use dioxus::prelude::*;
    /// let class = "my-class";
    /// let attrs = vec![];
    /// let color = "red";
    ///
    /// rsx! {
    ///     div {
    ///         class: "{class}",
    ///         ..attrs,
    ///         p {
    ///             color: "{color}",
    ///         }
    ///     }
    /// };
    /// ```
    ///
    /// Would be represented as:
    /// ```text
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
        static EMPTY_NODE_CURSORS: &[TemplateCursor] = &[TemplateCursor::new(&[0])];
        static EMPTY_TEMPLATE: Template = Template::new(&[], EMPTY_NODE_CURSORS, &[]);
        let vnode = EMPTY_VNODE.with(|cell| {
            cell.get_or_init(move || {
                Rc::new(VNodeInner {
                    key: None,
                    dynamic_nodes: Box::new([DynamicNode::Fragment(Vec::new())]),
                    dynamic_attrs: Box::new([]),
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
        static ERROR_ANCHOR_NODE_CURSORS: &[TemplateCursor] = &[TemplateCursor::new(&[0])];
        static ERROR_ANCHOR_TEMPLATE: Template = Template::new(&[], ERROR_ANCHOR_NODE_CURSORS, &[]);
        let vnode = ERROR_ANCHOR_VNODE.with(|cell| {
            cell.get_or_init(move || {
                Rc::new(VNodeInner {
                    key: None,
                    dynamic_nodes: Box::new([DynamicNode::Text(VText {
                        value: String::new(),
                    })]),
                    dynamic_attrs: Box::new([]),
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
        dynamic_nodes: Box<[DynamicNode]>,
        dynamic_attrs: Box<[Box<[Attribute]>]>,
    ) -> Self {
        // The diff assumes every dynamic attribute slot is sorted by `(name, namespace)`. Named
        // attributes are trivially sorted (one entry per slot); spread attributes are user-provided
        // and the only realistic source of violations.
        #[cfg(debug_assertions)]
        for slot in &dynamic_attrs {
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

        Self {
            vnode: Rc::new(VNodeInner {
                key,
                template,
                dynamic_nodes,
                dynamic_attrs,
            }),
            mount: Cell::new(Self::UNMOUNTED_MOUNT),
        }
    }

    /// Load a root-level dynamic node slot at the given dynamic node index
    ///
    /// Returns [`None`] if the dynamic node is mounted under a static template node.
    pub fn dynamic_root(&self, idx: usize) -> Option<&DynamicNode> {
        self.template
            .node_cursors()
            .get(idx)
            .filter(|cursor| cursor.is_root_level_slot())
            .map(|_| &self.dynamic_nodes[idx])
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

        match &self.dynamic_nodes[dynamic_node_idx] {
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
        Self {
            vnode: Rc::new(VNodeInner {
                key: self.vnode.key.clone(),
                template: self.vnode.template,
                dynamic_nodes: self
                    .vnode
                    .dynamic_nodes
                    .iter()
                    .map(|node| match node {
                        DynamicNode::Fragment(nodes) => DynamicNode::Fragment(
                            nodes.iter().map(|node| node.deep_clone()).collect(),
                        ),
                        other => other.clone(),
                    })
                    .collect(),
                dynamic_attrs: self
                    .vnode
                    .dynamic_attrs
                    .iter()
                    .map(|attr| {
                        attr.iter()
                            .map(|attribute| attribute.deep_clone())
                            .collect()
                    })
                    .collect(),
            }),
            mount: Cell::new(Self::UNMOUNTED_MOUNT),
        }
    }

    /// Deep-clone the tree while preserving every per-node raw mount slot. Each
    /// `VNodeInner` is freshly allocated so the resulting tree's per-node
    /// `Cell<usize>` slots are independent from this one — diffing against
    /// the clone won't mutate this tree's mount state via the shared `Rc`.
    ///
    /// Used by `SuspenseBranch::root` to hand out a fresh tree per diff pass
    /// without losing the mount info the diff needs to talk to the renderer.
    pub(crate) fn deep_clone_preserving_mounts(&self) -> Self {
        Self {
            vnode: Rc::new(VNodeInner {
                key: self.vnode.key.clone(),
                template: self.vnode.template,
                dynamic_nodes: self
                    .vnode
                    .dynamic_nodes
                    .iter()
                    .map(|node| match node {
                        DynamicNode::Fragment(nodes) => DynamicNode::Fragment(
                            nodes
                                .iter()
                                .map(|node| node.deep_clone_preserving_mounts())
                                .collect(),
                        ),
                        other => other.clone(),
                    })
                    .collect(),
                dynamic_attrs: self
                    .vnode
                    .dynamic_attrs
                    .iter()
                    .map(|attr| {
                        attr.iter()
                            .map(|attribute| attribute.deep_clone())
                            .collect()
                    })
                    .collect(),
            }),
            mount: Cell::new(self.mount.get()),
        }
    }
}

type StaticStr = &'static str;
type StaticCursorArray = &'static [TemplateCursor];
type StaticTemplateArray = &'static [TemplateNode];
type StaticTemplateAttributeArray = &'static [TemplateAttribute];

/// A cursor into a template's static children, or into a dynamic insertion slot.
///
/// For a static node, every segment indexes the nth static child at that depth.
/// For a dynamic node slot, the final segment is the insertion index among the
/// static children of the parent identified by the preceding segments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
pub struct TemplateCursor(
    #[cfg_attr(
        feature = "serialize",
        serde(deserialize_with = "deserialize_byte_slice_leaky")
    )]
    &'static [u8],
);

impl TemplateCursor {
    /// Create a new template cursor from a static byte slice.
    pub const fn new(cursor: &'static [u8]) -> Self {
        Self(cursor)
    }

    /// Return the cursor as a slice.
    pub const fn as_slice(self) -> &'static [u8] {
        self.0
    }

    /// Return true if this cursor points at a root-level dynamic slot.
    pub const fn is_root_level_slot(self) -> bool {
        self.0.len() == 1
    }

    /// Split this dynamic slot cursor into `(parent_cursor, insertion_index)`.
    pub fn split_slot(self) -> (&'static [u8], usize) {
        let (index, parent) = self
            .0
            .split_last()
            .expect("dynamic slot cursors must include an insertion index");
        (parent, *index as usize)
    }

    /// Return the parent cursor of this dynamic slot.
    pub fn slot_parent(self) -> &'static [u8] {
        self.split_slot().0
    }

    /// Return true if this static cursor is equal to or beneath `ancestor`.
    pub fn is_descendant_of_static(self, ancestor: TemplateCursor) -> bool {
        let ancestor = ancestor.as_slice();
        ancestor.len() <= self.0.len() && ancestor == &self.0[..ancestor.len()]
    }

    /// Return true if this dynamic slot is mounted inside `ancestor`.
    pub fn slot_is_inside_static(self, ancestor: TemplateCursor) -> bool {
        let ancestor = ancestor.as_slice();
        let parent = self.slot_parent();
        ancestor.len() <= parent.len() && ancestor == &parent[..ancestor.len()]
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
    /// The list of template nodes that make up the template
    ///
    /// Unlike react, calls to `rsx!` can have multiple roots. This list supports that paradigm.
    #[cfg_attr(feature = "serialize", serde(deserialize_with = "deserialize_leaky"))]
    roots: StaticTemplateArray,

    /// The insertion cursor for each dynamic node.
    #[cfg_attr(
        feature = "serialize",
        serde(deserialize_with = "deserialize_cursors_leaky")
    )]
    node_cursors: StaticCursorArray,

    /// The static-node cursor for each dynamic attribute.
    #[cfg_attr(
        feature = "serialize",
        serde(deserialize_with = "deserialize_cursors_leaky", bound = "")
    )]
    attr_cursors: StaticCursorArray,

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
    /// Create a new Template with the given roots, node cursors, and attribute cursors.
    /// The hash is computed automatically from the template content.
    pub const fn new(
        roots: &'static [TemplateNode],
        node_cursors: &'static [TemplateCursor],
        attr_cursors: &'static [TemplateCursor],
    ) -> Self {
        Self {
            roots,
            node_cursors,
            attr_cursors,
            hash: Self::compute_hash(roots, node_cursors, attr_cursors),
        }
    }

    /// Get the template nodes that make up this template.
    pub const fn roots(&self) -> &'static [TemplateNode] {
        self.roots
    }

    /// Get the insertion cursors for each dynamic node.
    pub const fn node_cursors(&self) -> &'static [TemplateCursor] {
        self.node_cursors
    }

    /// Get the static-node cursors for each dynamic attribute.
    pub const fn attr_cursors(&self) -> &'static [TemplateCursor] {
        self.attr_cursors
    }

    pub(crate) fn node_at_cursor(&self, cursor: TemplateCursor) -> Option<&'static TemplateNode> {
        let (root_idx, child_cursor) = cursor.as_slice().split_first()?;
        self.roots
            .get(*root_idx as usize)?
            .node_at_child_cursor(child_cursor)
    }

    /// Compute a content-based hash of template structure.
    /// This is const so it can be used both at compile time and runtime.
    const fn compute_hash(
        roots: &[TemplateNode],
        node_cursors: &[TemplateCursor],
        attr_cursors: &[TemplateCursor],
    ) -> u64 {
        use xxhash_rust::const_xxh64::xxh64;

        const fn hash_template_node(node: &TemplateNode, seed: u64) -> u64 {
            match node {
                TemplateNode::Element {
                    tag,
                    namespace,
                    attrs,
                    children,
                } => {
                    let mut h = xxh64(tag.as_bytes(), seed);
                    if let Some(ns) = *namespace {
                        h = xxh64(ns.as_bytes(), h);
                    }

                    // Hash attributes (already in deterministic order from macro)
                    let mut i = 0;
                    while i < attrs.len() {
                        h = match &attrs[i] {
                            TemplateAttribute::Static {
                                name,
                                value,
                                namespace,
                            } => {
                                let mut new_h = xxh64(name.as_bytes(), h);
                                new_h = xxh64(value.as_bytes(), new_h);
                                if let Some(ns) = *namespace {
                                    new_h = xxh64(ns.as_bytes(), new_h);
                                }
                                new_h
                            }
                            TemplateAttribute::Dynamic { id } => {
                                xxh64(&(*id as u64).to_le_bytes(), xxh64(&[0xFE], h))
                            }
                        };
                        i += 1;
                    }

                    // Hash children
                    let mut i = 0;
                    while i < children.len() {
                        h = hash_template_node(&children[i], h);
                        i += 1;
                    }

                    h
                }
                TemplateNode::Text { text } => xxh64(text.as_bytes(), seed),
            }
        }

        let mut hash = 0u64;

        // Hash roots
        let mut i = 0;
        while i < roots.len() {
            hash = hash_template_node(&roots[i], hash);
            i += 1;
        }

        // Hash node cursors (mixed with a section marker so they can't collapse into attr_cursors)
        hash = xxh64(&[0xA1], hash);
        let mut i = 0;
        while i < node_cursors.len() {
            hash = xxh64(node_cursors[i].as_slice(), hash);
            i += 1;
        }

        // Hash attr cursors
        hash = xxh64(&[0xA2], hash);
        let mut i = 0;
        while i < attr_cursors.len() {
            hash = xxh64(attr_cursors[i].as_slice(), hash);
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
fn deserialize_byte_slice_leaky<'a, 'de, D>(deserializer: D) -> Result<&'static [u8], D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    let deserialized = Vec::<u8>::deserialize(deserializer)?;
    Ok(&*Box::leak(deserialized.into_boxed_slice()))
}

#[cfg(feature = "serialize")]
pub(crate) fn deserialize_cursors_leaky<'a, 'de, D>(
    deserializer: D,
) -> Result<&'static [TemplateCursor], D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    let deserialized = Vec::<Vec<u8>>::deserialize(deserializer)?;
    let deserialized = deserialized
        .into_iter()
        .map(|v| TemplateCursor(&*Box::leak(v.into_boxed_slice())))
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
        self.roots.is_empty()
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
        #[cfg_attr(
            feature = "serialize",
            serde(deserialize_with = "deserialize_string_leaky")
        )]
        tag: StaticStr,

        /// The namespace of the element
        ///
        /// In HTML, this would be a valid URI that defines a namespace for all elements below it
        /// SVG is an example of this namespace
        #[cfg_attr(
            feature = "serialize",
            serde(deserialize_with = "deserialize_option_leaky")
        )]
        namespace: Option<StaticStr>,

        /// A list of possibly dynamic attributes for this element
        ///
        /// An attribute on a DOM node, such as `id="my-thing"` or `href="https://example.com"`.
        #[cfg_attr(
            feature = "serialize",
            serde(deserialize_with = "deserialize_leaky", bound = "")
        )]
        attrs: StaticTemplateAttributeArray,

        /// A list of template nodes that define another set of template nodes
        #[cfg_attr(feature = "serialize", serde(deserialize_with = "deserialize_leaky"))]
        children: StaticTemplateArray,
    },

    /// This template node is just a piece of static text
    Text {
        /// The actual text
        #[cfg_attr(
            feature = "serialize",
            serde(deserialize_with = "deserialize_string_leaky", bound = "")
        )]
        text: StaticStr,
    },
}

impl TemplateNode {
    pub(crate) fn element_child(&self, child_idx: usize) -> &'static TemplateNode {
        let TemplateNode::Element { children, .. } = self else {
            unreachable!("template attribute paths only pass through elements")
        };
        &children[child_idx]
    }

    pub(crate) fn element_attrs(&self) -> &'static [TemplateAttribute] {
        let TemplateNode::Element { attrs, .. } = self else {
            unreachable!("template attribute paths only point to elements")
        };
        attrs
    }

    pub(crate) fn node_at_child_cursor(
        &'static self,
        cursor: &[u8],
    ) -> Option<&'static TemplateNode> {
        let mut node = self;
        for child_idx in cursor {
            let TemplateNode::Element { children, .. } = node else {
                return None;
            };
            node = children.get(*child_idx as usize)?;
        }
        Some(node)
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

/// An attribute of the TemplateNode, created at compile time
#[derive(Clone, Copy, Debug, PartialEq, Hash, Eq, PartialOrd, Ord)]
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
        #[cfg_attr(
            feature = "serialize",
            serde(deserialize_with = "deserialize_string_leaky", bound = "")
        )]
        name: StaticStr,

        /// The value of this attribute, known at compile time
        ///
        /// Currently this only accepts &str, so values, even if they're known at compile time, are not known
        #[cfg_attr(
            feature = "serialize",
            serde(deserialize_with = "deserialize_string_leaky", bound = "")
        )]
        value: StaticStr,

        /// The namespace of this attribute. Does not exist in the HTML spec
        #[cfg_attr(
            feature = "serialize",
            serde(deserialize_with = "deserialize_option_leaky", bound = "")
        )]
        namespace: Option<StaticStr>,
    },

    /// The attribute in this position is actually determined dynamically at runtime
    ///
    /// This is the index into the dynamic_attributes field on the container VNode
    Dynamic {
        /// The index
        id: usize,
    },
}

#[doc(hidden)]
/// Sort static template attributes by their emitted name while leaving dynamic attributes in place.
///
/// The diffing code binary-searches the static prefix by `TemplateAttribute::Static::name` when a
/// dynamic spread stops overriding a static value. The RSX syntax name is not always the emitted
/// DOM name (`r#as` emits `as`, `http_equiv` emits `http-equiv`), so this runs after macro
/// expansion has produced the actual static names.
pub const fn sort_template_attributes<const N: usize>(
    mut attrs: [TemplateAttribute; N],
) -> [TemplateAttribute; N] {
    // The macro emits static attrs first and dynamic attrs second. Only the static prefix is
    // sorted because dynamic attrs are addressed by id from the VNode's dynamic attribute list.
    let mut static_len = 0;
    while static_len < N {
        match attrs[static_len] {
            TemplateAttribute::Static { .. } => static_len += 1,
            TemplateAttribute::Dynamic { .. } => break,
        }
    }

    // Attribute lists are small, and insertion sort is const-friendly on stable Rust.
    let mut i = 1;
    while i < static_len {
        let mut j = i;
        while j > 0 && template_attribute_name_less(attrs[j], attrs[j - 1]) {
            let previous = attrs[j - 1];
            attrs[j - 1] = attrs[j];
            attrs[j] = previous;
            j -= 1;
        }
        i += 1;
    }

    attrs
}

const fn template_attribute_name_less(left: TemplateAttribute, right: TemplateAttribute) -> bool {
    match (left, right) {
        (
            TemplateAttribute::Static { name: left, .. },
            TemplateAttribute::Static { name: right, .. },
        ) => static_str_less(left, right),
        _ => false,
    }
}

const fn static_str_less(left: StaticStr, right: StaticStr) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    let mut idx = 0;

    while idx < left.len() && idx < right.len() {
        if left[idx] < right[idx] {
            return true;
        }
        if left[idx] > right[idx] {
            return false;
        }
        idx += 1;
    }

    left.len() < right.len()
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

    /// Create a new deep clone of this attribute
    pub(crate) fn deep_clone(&self) -> Self {
        Attribute {
            name: self.name,
            namespace: self.namespace,
            volatile: self.volatile,
            value: self.value.clone(),
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

impl IntoAttributeValue for f32 {
    fn into_value(self) -> AttributeValue {
        AttributeValue::Float(self as _)
    }
}
impl IntoAttributeValue for f64 {
    fn into_value(self) -> AttributeValue {
        AttributeValue::Float(self)
    }
}

impl IntoAttributeValue for i8 {
    fn into_value(self) -> AttributeValue {
        AttributeValue::Int(self as _)
    }
}
impl IntoAttributeValue for i16 {
    fn into_value(self) -> AttributeValue {
        AttributeValue::Int(self as _)
    }
}
impl IntoAttributeValue for i32 {
    fn into_value(self) -> AttributeValue {
        AttributeValue::Int(self as _)
    }
}
impl IntoAttributeValue for i64 {
    fn into_value(self) -> AttributeValue {
        AttributeValue::Int(self)
    }
}
impl IntoAttributeValue for isize {
    fn into_value(self) -> AttributeValue {
        AttributeValue::Int(self as _)
    }
}
impl IntoAttributeValue for i128 {
    fn into_value(self) -> AttributeValue {
        AttributeValue::Int(self as _)
    }
}

impl IntoAttributeValue for u8 {
    fn into_value(self) -> AttributeValue {
        AttributeValue::Int(self as _)
    }
}
impl IntoAttributeValue for u16 {
    fn into_value(self) -> AttributeValue {
        AttributeValue::Int(self as _)
    }
}
impl IntoAttributeValue for u32 {
    fn into_value(self) -> AttributeValue {
        AttributeValue::Int(self as _)
    }
}
impl IntoAttributeValue for u64 {
    fn into_value(self) -> AttributeValue {
        AttributeValue::Int(self as _)
    }
}
impl IntoAttributeValue for usize {
    fn into_value(self) -> AttributeValue {
        AttributeValue::Int(self as _)
    }
}
impl IntoAttributeValue for u128 {
    fn into_value(self) -> AttributeValue {
        AttributeValue::Int(self as _)
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
