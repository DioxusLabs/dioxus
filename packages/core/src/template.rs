use dioxus_const_vec::ConstVec;
use std::num::NonZeroU128;

type StaticTemplateOpArray = &'static [TemplateOp];
type StaticTemplateStringArray = &'static [&'static str];

/// A compact path from a template root to a static node or dynamic attribute.
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
    pub const fn len(self) -> usize {
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
    pub(crate) fn segment(self, index: usize) -> u8 {
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

    /// Return true if this compact path starts with `ancestor`.
    pub fn starts_with(self, ancestor: TemplatePath) -> bool {
        let self_len = self.bit_len();
        let ancestor_len = ancestor.bit_len();
        ancestor.path == 0
            || (ancestor_len <= self_len
                && (self.path >> (self_len - ancestor_len)) == ancestor.path)
    }

    /// Return the number of raw child/sibling bits in this path.
    pub(crate) fn bit_len(self) -> u32 {
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

/// A tagged dynamic node slot target.
///
/// The low bit is the target kind. The remaining high bits are a [`TemplatePath`] payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct TemplateSlotPath(NonZeroU128);

/// The resolved renderer target for a dynamic node slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateSlotTarget {
    /// Insert before a static node.
    BeforeStatic(TemplatePath),
    /// Append to a static parent. An empty path means append at the vnode's render-parent site.
    AppendChildren(TemplatePath),
}

impl TemplateSlotPath {
    const TARGET_APPEND_CHILDREN: u128 = 1;
    const MAX_PAYLOAD: u128 = u128::MAX >> 1;

    const fn new(bits: u128) -> Self {
        match NonZeroU128::new(bits) {
            Some(bits) => Self(bits),
            None => panic!("template slot path must be non-zero"),
        }
    }

    const fn encode_payload(path: TemplatePath) -> u128 {
        let payload = path.bits();
        if payload > Self::MAX_PAYLOAD {
            panic!("template slot path payload exceeds packed capacity");
        }
        payload << 1
    }

    /// Create a dynamic slot target before a static node.
    pub const fn before_static(path: TemplatePath) -> Self {
        if path.is_empty() {
            panic!("before-static slot target requires a static node path");
        }
        Self::new(Self::encode_payload(path))
    }

    /// Create a dynamic slot target that appends to a parent.
    pub const fn append_children(path: TemplatePath) -> Self {
        Self::new(Self::encode_payload(path) | Self::TARGET_APPEND_CHILDREN)
    }

    /// Create a slot path from raw bits.
    pub const fn from_bits(bits: u128) -> Self {
        Self::new(bits)
    }

    /// Return the raw tagged bits.
    pub const fn bits(self) -> u128 {
        self.0.get()
    }

    /// Decode the target kind and path payload.
    pub const fn target(self) -> TemplateSlotTarget {
        let bits = self.bits();
        let path = TemplatePath::from_bits(bits >> 1);
        if bits & Self::TARGET_APPEND_CHILDREN == Self::TARGET_APPEND_CHILDREN {
            TemplateSlotTarget::AppendChildren(path)
        } else {
            TemplateSlotTarget::BeforeStatic(path)
        }
    }

    /// Return true if this slot is mounted at the vnode root level.
    pub const fn is_root_level(self) -> bool {
        match self.target() {
            TemplateSlotTarget::BeforeStatic(path) => path.len() == 1,
            TemplateSlotTarget::AppendChildren(path) => path.is_empty(),
        }
    }

    /// Return the static parent path used for containment checks.
    pub const fn static_parent(self) -> TemplatePath {
        match self.target() {
            TemplateSlotTarget::BeforeStatic(path) => path.parent(),
            TemplateSlotTarget::AppendChildren(path) => path,
        }
    }

    /// Return the root index of the static node or parent this slot targets.
    pub fn root_index(self) -> Option<usize> {
        match self.target() {
            TemplateSlotTarget::BeforeStatic(path) => Some(path.segment(0) as usize),
            TemplateSlotTarget::AppendChildren(path) => {
                (!path.is_empty()).then(|| path.segment(0) as usize)
            }
        }
    }

    /// Return the fill-order depth for this slot.
    pub const fn fill_depth(self) -> usize {
        match self.target() {
            TemplateSlotTarget::BeforeStatic(path) => path.len(),
            TemplateSlotTarget::AppendChildren(path) => path.len() + 1,
        }
    }

    /// Return true if this slot is mounted inside `ancestor`.
    pub fn is_inside_static(self, ancestor: TemplatePath) -> bool {
        self.static_parent().starts_with(ancestor)
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
        let path = <u128 as serde::Deserialize>::deserialize(deserializer)?;
        Ok(Self { path })
    }
}

/// Static attribute namespace information in a raw template tape.
#[doc(hidden)]
pub type TemplateRawAttrNamespace = Option<&'static str>;

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
    /// A text node marker. The next op is a [`Self::Static`] string reference.
    Text,
    /// A static string pool reference.
    Static(u16),
}

impl TemplateOp {
    const ENTER_MAX_CODE: u16 = 0x7fff;
    const ATTR_CODE: u16 = 0x8000;
    const ATTR_CUSTOM_NS_CODE: u16 = 0x8001;
    const TEXT_CODE: u16 = 0x8002;
    const STATIC_BASE: u16 = 0x8003;
    const MAX_CAP: usize = 16_383;

    /// Create a packed enter op.
    pub(crate) const fn enter(skip: u16, namespace: bool) -> Self {
        if skip as usize > Self::MAX_CAP {
            panic!("op skip exceeds packed op capacity");
        }
        Self((skip << 1) | namespace as u16)
    }

    /// Create a packed static attribute op.
    pub(crate) const fn attr(namespace: bool) -> Self {
        if namespace {
            Self(Self::ATTR_CUSTOM_NS_CODE)
        } else {
            Self(Self::ATTR_CODE)
        }
    }

    /// Create a packed static attribute op with a following custom namespace string.
    pub(crate) const fn attr_custom_namespace() -> Self {
        Self(Self::ATTR_CUSTOM_NS_CODE)
    }

    /// Create a packed text marker op.
    pub(crate) const fn text() -> Self {
        Self(Self::TEXT_CODE)
    }

    /// Create a packed static string reference op.
    pub(crate) const fn static_text(id: u16) -> Self {
        if id as usize >= Self::MAX_CAP {
            panic!("static op id exceeds packed op capacity");
        }
        Self(Self::STATIC_BASE + id)
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
        } else if self.0 == Self::ATTR_CUSTOM_NS_CODE {
            DecodedTemplateOp::Attr {
                namespace: DecodedTemplateAttrNamespace::Custom,
            }
        } else if self.0 == Self::TEXT_CODE {
            DecodedTemplateOp::Text
        } else {
            DecodedTemplateOp::Static(self.0 - Self::STATIC_BASE)
        }
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
}

/// Sentinel `op` value marking a [`TemplateAnchor`] for a root-level dynamic node slot, which has no
/// enclosing static element.
pub(crate) const ROOT_ANCHOR_OP: u16 = u16::MAX;

#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum TemplateAnchorKind {
    Attr,
    Node,
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct TemplateAnchor {
    op: u16,
    kind: TemplateAnchorKind,
    path: u128,
    value_start: u16,
    value_count: u16,
}

impl TemplateAnchor {
    pub const fn new(op: u16, path: TemplateSlotPath, value_start: u16, value_count: u16) -> Self {
        Self::node(op, path, value_start, value_count)
    }

    const fn attr(op: u16, path: TemplatePath, value_start: u16, value_count: u16) -> Self {
        if value_count == 0 {
            panic!("template anchors must cover at least one dynamic value");
        }
        Self {
            op,
            kind: TemplateAnchorKind::Attr,
            path: path.bits(),
            value_start,
            value_count,
        }
    }

    const fn node(op: u16, path: TemplateSlotPath, value_start: u16, value_count: u16) -> Self {
        if value_count == 0 {
            panic!("template anchors must cover at least one dynamic value");
        }
        Self {
            op,
            kind: TemplateAnchorKind::Node,
            path: path.bits(),
            value_start,
            value_count,
        }
    }

    const fn single_attr(op: u16, path: TemplatePath, value_start: u16) -> Self {
        Self::attr(op, path, value_start, 1)
    }

    const fn single_node(op: u16, path: TemplateSlotPath, value_start: u16) -> Self {
        Self::node(op, path, value_start, 1)
    }

    pub const fn root_node(value_index: u16, root_idx: usize, appends: bool) -> Self {
        let slot = if appends {
            TemplateSlotPath::append_children(TemplatePath::empty())
        } else {
            TemplateSlotPath::before_static(TemplatePath::root(root_idx))
        };
        Self::single_node(ROOT_ANCHOR_OP, slot, value_index)
    }

    const fn kind(self) -> TemplateAnchorKind {
        self.kind
    }

    const fn path_bits(self) -> u128 {
        self.path
    }

    pub fn element_op(self) -> Option<usize> {
        (self.op != ROOT_ANCHOR_OP).then_some(self.op as usize)
    }

    pub fn is_root_level(self) -> bool {
        self.kind == TemplateAnchorKind::Node && self.op == ROOT_ANCHOR_OP
    }

    pub(crate) const fn path(self) -> TemplatePath {
        TemplatePath::from_bits(self.path)
    }

    pub const fn slot_path(self) -> TemplateSlotPath {
        TemplateSlotPath::from_bits(self.path)
    }

    pub const fn slot_target(self) -> TemplateSlotTarget {
        self.slot_path().target()
    }

    pub fn value_start(self) -> usize {
        self.value_start as usize
    }

    pub fn value_count(self) -> usize {
        self.value_count as usize
    }

    pub fn values(self) -> std::ops::Range<usize> {
        self.value_start as usize..(self.value_start as usize + self.value_count as usize)
    }

    const fn same_slot_bits(self, op: u16, kind: TemplateAnchorKind, path: u128) -> bool {
        self.op == op
            && matches!(
                (self.kind, kind),
                (TemplateAnchorKind::Attr, TemplateAnchorKind::Attr)
                    | (TemplateAnchorKind::Node, TemplateAnchorKind::Node)
            )
            && self.path == path
    }

    const fn should_fill_before(self, other: Self) -> bool {
        let self_depth = if matches!(self.kind, TemplateAnchorKind::Node) {
            self.slot_path().fill_depth()
        } else {
            self.path().len()
        };
        let other_depth = if matches!(other.kind, TemplateAnchorKind::Node) {
            other.slot_path().fill_depth()
        } else {
            other.path().len()
        };
        if self_depth != other_depth {
            return self_depth > other_depth;
        }

        self.value_start > other.value_start
    }
}

/// Maximum packed template storage capacity.
pub(crate) const TEMPLATE_STORAGE_MAX_CAP: usize = TemplateOp::MAX_CAP;

const TEMPLATE_PATH_STACK_CAP: usize = 129;

/// Const storage for a lowered raw template.
///
/// The RSX macro emits a `static TemplateStorage<OPS, STRINGS, DYNAMICS>` from a
/// raw operation tape, then calls [`Self::as_template`] to expose the compact [`Template`] used by
/// the runtime.
#[derive(Clone, Copy)]
pub(crate) struct TemplateStorage<
    const OPS_CAP: usize = TEMPLATE_STORAGE_MAX_CAP,
    const STRING_CAP: usize = TEMPLATE_STORAGE_MAX_CAP,
    const DYNAMIC_CAP: usize = TEMPLATE_STORAGE_MAX_CAP,
> {
    ops: ConstVec<TemplateOp, OPS_CAP>,
    strings: ConstVec<&'static str, STRING_CAP>,
    anchors: ConstVec<TemplateAnchor, DYNAMIC_CAP>,
}

struct RawTemplateLoweringCursor {
    enter_stack: [usize; TEMPLATE_PATH_STACK_CAP],
    element_paths: [TemplatePath; TEMPLATE_PATH_STACK_CAP],
    next_paths: [TemplatePath; TEMPLATE_PATH_STACK_CAP],
    stack_pointer: usize,
}

impl RawTemplateLoweringCursor {
    const fn new() -> Self {
        let mut next_paths = [TemplatePath::empty(); TEMPLATE_PATH_STACK_CAP];
        next_paths[0] = TemplatePath::root(0);
        Self {
            enter_stack: [0; TEMPLATE_PATH_STACK_CAP],
            element_paths: [TemplatePath::empty(); TEMPLATE_PATH_STACK_CAP],
            next_paths,
            stack_pointer: 0,
        }
    }

    const fn open_element(&mut self, enter_index: usize) {
        if self.stack_pointer + 1 >= TEMPLATE_PATH_STACK_CAP {
            panic!("template path stack capacity exceeded");
        }
        let path = self.next_paths[self.stack_pointer];
        self.next_paths[self.stack_pointer] = path.next_sibling();
        self.element_paths[self.stack_pointer] = path;
        self.enter_stack[self.stack_pointer] = enter_index;
        self.next_paths[self.stack_pointer + 1] = path.next_child();
        self.stack_pointer += 1;
    }

    const fn close_element(&mut self) -> usize {
        if self.stack_pointer == 0 {
            panic!("template close op without matching open op");
        }
        self.stack_pointer -= 1;
        self.enter_stack[self.stack_pointer]
    }

    const fn current_element_path(&self) -> TemplatePath {
        if self.stack_pointer == 0 {
            panic!("dynamic attr raw op without an open element");
        }
        self.element_paths[self.stack_pointer - 1]
    }

    const fn current_element_op(&self) -> u16 {
        if self.stack_pointer == 0 {
            panic!("dynamic attr raw op without an open element");
        }
        self.enter_stack[self.stack_pointer - 1] as u16
    }

    const fn node_anchor_op(&self) -> u16 {
        if self.stack_pointer == 0 {
            ROOT_ANCHOR_OP
        } else {
            self.enter_stack[self.stack_pointer - 1] as u16
        }
    }

    const fn next_node_path(&mut self) -> TemplatePath {
        let path = self.next_paths[self.stack_pointer];
        self.next_paths[self.stack_pointer] = path.next_sibling();
        path
    }

    const fn next_slot_path(
        &self,
        raw: &'static [TemplateRawOp],
        index: usize,
    ) -> TemplateSlotPath {
        if self.dynamic_node_has_following_static_at_parent(raw, index) {
            return TemplateSlotPath::before_static(self.next_paths[self.stack_pointer]);
        }

        if self.stack_pointer == 0 {
            TemplateSlotPath::append_children(TemplatePath::empty())
        } else {
            TemplateSlotPath::append_children(self.element_paths[self.stack_pointer - 1])
        }
    }

    const fn finish(&self) {
        if self.stack_pointer != 0 {
            panic!("template raw ops ended with unclosed elements");
        }
    }

    const fn dynamic_node_has_following_static_at_parent(
        &self,
        raw: &'static [TemplateRawOp],
        index: usize,
    ) -> bool {
        let parent_depth = self.stack_pointer;
        let mut depth = parent_depth;
        let mut cursor = index + 1;

        while cursor < raw.len() {
            match raw[cursor] {
                TemplateRawOp::OpenElement { .. } => {
                    if depth == parent_depth {
                        return true;
                    }
                    depth += 1;
                }
                TemplateRawOp::StaticText { .. } => {
                    if depth == parent_depth {
                        return true;
                    }
                }
                TemplateRawOp::CloseElement => {
                    if depth == parent_depth {
                        return false;
                    }
                    depth -= 1;
                }
                TemplateRawOp::StaticAttr { .. }
                | TemplateRawOp::DynamicAttr
                | TemplateRawOp::DynamicNode => {}
            }
            cursor += 1;
        }

        false
    }
}

macro_rules! lower_raw_template {
    ($raw:expr, $builder:ident) => {{
        let mut cursor = RawTemplateLoweringCursor::new();
        let mut index = 0usize;
        while index < $raw.len() {
            match $raw[index] {
                TemplateRawOp::OpenElement { tag, namespace } => {
                    cursor.open_element($builder.ops_len());
                    $builder.push_op(TemplateOp::enter(0, namespace.is_some()));
                    $builder.push_static(tag);
                    if let Some(namespace) = namespace {
                        $builder.push_static(namespace);
                    }
                }
                TemplateRawOp::CloseElement => {
                    let enter_index = cursor.close_element();
                    let namespace = $builder.op_at(enter_index).has_namespace();
                    let skip = $builder.ops_len() - enter_index;
                    if skip > TemplateOp::MAX_CAP {
                        panic!("template op skip exceeds packed op capacity");
                    }
                    $builder.set_op(enter_index, TemplateOp::enter(skip as u16, namespace));
                }
                TemplateRawOp::StaticAttr {
                    name,
                    value,
                    namespace,
                } => {
                    $builder.push_op(TemplateOp::attr(namespace.is_some()));
                    $builder.push_static(name);
                    $builder.push_static(value);
                    if let Some(namespace) = namespace {
                        $builder.push_static(namespace);
                    }
                }
                TemplateRawOp::DynamicAttr => {
                    $builder.push_attr_anchor(
                        cursor.current_element_op(),
                        cursor.current_element_path(),
                    );
                }
                TemplateRawOp::StaticText { value } => {
                    let _ = cursor.next_node_path();
                    $builder.push_op(TemplateOp::text());
                    $builder.push_static(value);
                }
                TemplateRawOp::DynamicNode => {
                    let path = cursor.next_slot_path($raw, index);
                    $builder.push_node_anchor(cursor.node_anchor_op(), path);
                }
            }
            index += 1;
        }
        cursor.finish();
    }};
}

impl<const OPS_CAP: usize, const STRING_CAP: usize, const DYNAMIC_CAP: usize>
    TemplateStorage<OPS_CAP, STRING_CAP, DYNAMIC_CAP>
{
    /// Lower a raw template tape into packed storage in const context.
    pub(crate) const fn build(raw: &'static [TemplateRawOp]) -> Self {
        let mut storage = Self {
            ops: ConstVec::new_with_max_size(),
            strings: ConstVec::new_with_max_size(),
            anchors: ConstVec::new_with_max_size(),
        };

        lower_raw_template!(raw, storage);
        storage.sort_anchors_in_fill_order();
        storage
    }

    /// Return this storage as a compact template.
    pub(crate) const fn as_template(&'static self) -> Template {
        Template::new(
            self.ops.as_slice(),
            self.strings.as_slice(),
            self.anchors.as_slice(),
        )
    }

    const fn push_static(&mut self, value: &'static str) {
        let id = self.strings.len();
        if id >= TemplateOp::MAX_CAP {
            panic!("static op id exceeds packed op capacity");
        }
        self.strings.push(value);
        self.ops.push(TemplateOp::static_text(id as u16));
    }

    const fn ops_len(&self) -> usize {
        self.ops.len()
    }

    const fn op_at(&self, index: usize) -> TemplateOp {
        self.ops.at(index)
    }

    const fn push_op(&mut self, op: TemplateOp) {
        self.ops.push(op);
    }

    const fn set_op(&mut self, index: usize, op: TemplateOp) {
        self.ops.set(index, op);
    }

    const fn push_attr_anchor(&mut self, op: u16, path: TemplatePath) {
        self.push_anchor_bits(op, path.bits(), TemplateAnchorKind::Attr);
    }

    const fn push_node_anchor(&mut self, op: u16, path: TemplateSlotPath) {
        self.push_anchor_bits(op, path.bits(), TemplateAnchorKind::Node);
    }

    const fn push_anchor_bits(&mut self, op: u16, path: u128, kind: TemplateAnchorKind) {
        let len = self.anchors.len();
        if len > 0 {
            let last = self.anchors.at(len - 1);
            if last.same_slot_bits(op, kind, path) {
                self.anchors.set(
                    len - 1,
                    TemplateAnchor {
                        value_count: last.value_count + 1,
                        ..last
                    },
                );
                return;
            }
        }
        let mut i = 0;
        while i < len {
            if self.anchors.at(i).same_slot_bits(op, kind, path) {
                panic!(
                    "dynamic values for a template anchor must be contiguous (attributes must precede children)"
                );
            }
            i += 1;
        }
        let value_start = if len == 0 {
            0
        } else {
            let last = self.anchors.at(len - 1);
            last.value_start + last.value_count
        };
        let anchor = match kind {
            TemplateAnchorKind::Attr => {
                TemplateAnchor::single_attr(op, TemplatePath::from_bits(path), value_start)
            }
            TemplateAnchorKind::Node => {
                TemplateAnchor::single_node(op, TemplateSlotPath::from_bits(path), value_start)
            }
        };
        self.anchors.push(anchor);
    }

    const fn sort_anchors_in_fill_order(&mut self) {
        let len = self.anchors.len();
        let mut index = 0;
        while index < len {
            let mut best = index;
            let mut candidate = index + 1;
            while candidate < len {
                if self
                    .anchors
                    .at(candidate)
                    .should_fill_before(self.anchors.at(best))
                {
                    best = candidate;
                }
                candidate += 1;
            }
            if best != index {
                self.anchors.swap(index, best);
            }
            index += 1;
        }
    }
}

struct RuntimeTemplateBuilder {
    ops: Vec<TemplateOp>,
    strings: Vec<&'static str>,
    anchors: Vec<TemplateAnchor>,
}

impl RuntimeTemplateBuilder {
    fn new() -> Self {
        Self {
            ops: Vec::new(),
            strings: Vec::new(),
            anchors: Vec::new(),
        }
    }

    fn ops_len(&self) -> usize {
        self.ops.len()
    }

    fn op_at(&self, index: usize) -> TemplateOp {
        self.ops[index]
    }

    fn push_op(&mut self, op: TemplateOp) {
        self.ops.push(op);
    }

    fn set_op(&mut self, index: usize, op: TemplateOp) {
        self.ops[index] = op;
    }

    fn push_static(&mut self, value: &'static str) {
        let id = self.strings.len();
        assert!(
            id < TemplateOp::MAX_CAP,
            "static op id exceeds packed op capacity"
        );
        self.strings.push(value);
        self.ops.push(TemplateOp::static_text(id as u16));
    }

    fn push_attr_anchor(&mut self, op: u16, path: TemplatePath) {
        self.push_anchor_bits(op, path.bits(), TemplateAnchorKind::Attr);
    }

    fn push_node_anchor(&mut self, op: u16, path: TemplateSlotPath) {
        self.push_anchor_bits(op, path.bits(), TemplateAnchorKind::Node);
    }

    fn push_anchor_bits(&mut self, op: u16, path: u128, kind: TemplateAnchorKind) {
        if let Some(last) = self.anchors.last_mut() {
            if last.same_slot_bits(op, kind, path) {
                last.value_count += 1;
                return;
            }
        }
        assert!(
            !self
                .anchors
                .iter()
                .any(|a| a.same_slot_bits(op, kind, path)),
            "dynamic values for a template anchor must be contiguous (attributes must precede children)"
        );
        let value_start = self
            .anchors
            .last()
            .map_or(0, |a| a.value_start + a.value_count);
        let anchor = match kind {
            TemplateAnchorKind::Attr => {
                TemplateAnchor::single_attr(op, TemplatePath::from_bits(path), value_start)
            }
            TemplateAnchorKind::Node => {
                TemplateAnchor::single_node(op, TemplateSlotPath::from_bits(path), value_start)
            }
        };
        self.anchors.push(anchor);
    }

    fn into_template(self) -> Template {
        let mut anchors = self.anchors;
        anchors.sort_by(|left, right| {
            if left.should_fill_before(*right) {
                std::cmp::Ordering::Less
            } else if right.should_fill_before(*left) {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        });
        Template::new(
            Box::leak(self.ops.into_boxed_slice()),
            Box::leak(self.strings.into_boxed_slice()),
            Box::leak(anchors.into_boxed_slice()),
        )
    }
}

impl Template {
    /// Lower a raw template tape into a leaked runtime template.
    pub(crate) fn from_raw_ops(raw: &'static [TemplateRawOp]) -> Self {
        let mut builder = RuntimeTemplateBuilder::new();
        lower_raw_template!(raw, builder);
        builder.into_template()
    }
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
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
#[derive(Debug, Clone, Copy, Eq, PartialOrd, Ord)]
pub struct Template {
    /// Flat static template operations.
    #[cfg_attr(feature = "serialize", serde(deserialize_with = "deserialize_leaky"))]
    ops: StaticTemplateOpArray,

    /// Static strings referenced by [`TemplateOp::Static`].
    #[cfg_attr(
        feature = "serialize",
        serde(deserialize_with = "deserialize_strings_leaky")
    )]
    strings: StaticTemplateStringArray,

    /// Dynamic value groups in reverse breadth-first fill order, each anchored to a static element.
    #[cfg_attr(feature = "serialize", serde(deserialize_with = "deserialize_leaky"))]
    anchors: &'static [TemplateAnchor],

    /// Total number of runtime dynamic values this template expects.
    #[cfg_attr(feature = "serialize", serde(skip))]
    dynamic_value_count: u16,

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
    /// Create a new flat template with the given ops, strings, and dynamic anchors.
    /// The hash is computed automatically from the template content.
    pub(crate) const fn new(
        ops: &'static [TemplateOp],
        strings: StaticTemplateStringArray,
        anchors: &'static [TemplateAnchor],
    ) -> Self {
        Self::validate_anchors(anchors);
        Self {
            ops,
            strings,
            anchors,
            dynamic_value_count: Self::compute_dynamic_value_count(anchors),
            hash: Self::compute_hash(ops, strings, anchors),
        }
    }

    /// Get the flat template operations.
    pub(crate) const fn ops(&self) -> &'static [TemplateOp] {
        self.ops
    }

    /// Get the template static string pool.
    pub(crate) const fn strings(&self) -> &'static [&'static str] {
        self.strings
    }

    const fn validate_anchors(anchors: &[TemplateAnchor]) {
        let mut index = 0;
        let mut has_start = anchors.is_empty();
        while index < anchors.len() {
            let anchor = anchors[index];
            if anchor.value_count == 0 {
                panic!("template anchors must cover at least one dynamic value");
            }

            let start = anchor.value_start;
            let end = Self::anchor_value_end(anchor);
            if start == 0 {
                has_start = true;
            }

            let mut other_index = 0;
            while other_index < anchors.len() {
                if index != other_index {
                    let other = anchors[other_index];
                    let other_end = Self::anchor_value_end(other);
                    if start < other_end && other.value_start < end {
                        panic!("template anchor dynamic value ranges must not overlap");
                    }
                }
                other_index += 1;
            }

            if start != 0 {
                let mut has_predecessor = false;
                let mut predecessor_index = 0;
                while predecessor_index < anchors.len() && !has_predecessor {
                    has_predecessor = Self::anchor_value_end(anchors[predecessor_index]) == start;
                    predecessor_index += 1;
                }
                if !has_predecessor {
                    panic!("template anchor dynamic value ranges must be contiguous");
                }
            }

            index += 1;
        }

        if !has_start {
            panic!("template anchor dynamic value ranges must start at zero");
        }
    }

    /// Get dynamic value anchors in native fill order.
    pub(crate) const fn anchors(&self) -> &'static [TemplateAnchor] {
        self.anchors
    }

    pub(crate) fn anchors_in_document_order(
        &self,
    ) -> impl DoubleEndedIterator<Item = &'static TemplateAnchor> + '_ {
        (0..self.dynamic_value_count()).filter_map(move |idx| {
            self.anchors
                .iter()
                .find(|anchor| anchor.value_start() == idx)
        })
    }

    #[doc(hidden)]
    pub(crate) fn reorder_dynamic_values_from_document_order<T>(&self, values: Vec<T>) -> Vec<T> {
        let expected = self.dynamic_value_count();
        assert_eq!(
            values.len(),
            expected,
            "dynamic value count must match template"
        );
        values
    }

    /// Return the total number of dynamic values.
    pub(crate) fn dynamic_value_count(&self) -> usize {
        self.dynamic_value_count as usize
    }

    pub(crate) fn anchor_for_value(&self, idx: usize) -> Option<&'static TemplateAnchor> {
        self.anchors.iter().find(|a| a.values().contains(&idx))
    }

    /// Get the number of root positions in this template.
    pub(crate) fn root_count(&self) -> usize {
        let mut count = 0;
        let mut op = 0;
        while op < self.ops.len() {
            if self.is_static_node_op(op) {
                count += 1;
            }
            op = self.next_sibling_op(op);
        }
        count + self.root_level_anchor_count()
    }

    fn root_level_anchor_count(&self) -> usize {
        self.anchors.iter().filter(|a| a.is_root_level()).count()
    }

    /// Get a static string from this template's string pool.
    pub(crate) fn string(&self, id: u16) -> &'static str {
        self.strings[id as usize]
    }

    /// Decode an element op into its subtree length and namespace presence.
    pub(crate) fn enter_meta(&self, op: usize) -> Option<(usize, bool)> {
        match self.ops.get(op).map(|op| op.decode()) {
            Some(DecodedTemplateOp::Enter { skip, namespace }) => Some((skip as usize, namespace)),
            _ => None,
        }
    }

    /// Return the static string referenced by an op.
    pub(crate) fn static_string_at_op(&self, op: usize) -> Option<&'static str> {
        match self.ops.get(op).map(|op| op.decode()) {
            Some(DecodedTemplateOp::Static(id)) => Some(self.string(id)),
            _ => None,
        }
    }

    /// Return the tag and namespace for an element op.
    pub(crate) fn element_meta_at_op(
        &self,
        op: usize,
    ) -> Option<(&'static str, Option<&'static str>)> {
        let (_, has_namespace) = self.enter_meta(op)?;
        let tag = self.static_string_at_op(op + 1)?;
        let namespace = has_namespace
            .then(|| self.static_string_at_op(op + 2))
            .flatten();
        Some((tag, namespace))
    }

    /// Return the first child/attribute op inside an element.
    pub(crate) fn element_children_start(&self, op: usize) -> Option<usize> {
        let (_, has_namespace) = self.enter_meta(op)?;
        Some(op + if has_namespace { 3 } else { 2 })
    }

    /// Return the name, value, and namespace for a static attr op.
    pub(crate) fn static_attr_at_op(
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
            DecodedTemplateAttrNamespace::Custom => self.static_string_at_op(op + 3),
        };
        Some((name, value, namespace))
    }

    /// Return the text for a static `Text, Static` node marker.
    pub(crate) fn static_text_at_op(&self, op: usize) -> Option<&'static str> {
        (self.ops.get(op).map(|op| op.decode()) == Some(DecodedTemplateOp::Text))
            .then(|| self.static_string_at_op(op + 1))
            .flatten()
    }

    /// Return the number of ops used by a static attr at `op`.
    pub(crate) fn attr_op_len(&self, op: usize) -> Option<usize> {
        match self.ops.get(op).map(|op| op.decode()) {
            Some(DecodedTemplateOp::Attr {
                namespace: DecodedTemplateAttrNamespace::Custom,
            }) => Some(4),
            Some(DecodedTemplateOp::Attr { .. }) => Some(3),
            _ => None,
        }
    }

    /// Return the op immediately after an element subtree.
    pub(crate) fn element_end(&self, op: usize) -> Option<usize> {
        let (skip, _) = self.enter_meta(op)?;
        Some(op + skip)
    }

    fn element_attr_child_ops(&self, element_op: usize) -> Option<(usize, usize, usize)> {
        let attr_start = self.element_children_start(element_op)?;
        let mut cursor = attr_start;
        let end = self.element_end(element_op)?;
        while cursor < end {
            if let Some(len) = self.attr_op_len(cursor) {
                cursor += len;
            } else {
                break;
            }
        }
        Some((attr_start, cursor, end))
    }

    pub(crate) fn first_child_node_op(&self, element_op: usize) -> Option<usize> {
        Some(self.element_attr_child_ops(element_op)?.1)
    }

    /// Find a static attr fallback value for a key in an element.
    pub(crate) fn static_attr_value_for_key(
        &self,
        element_op: usize,
        key: (&'static str, Option<&'static str>),
    ) -> Option<&'static str> {
        let (mut cursor, end, _) = self.element_attr_child_ops(element_op)?;
        let mut found = None;
        while cursor < end {
            if let Some((name, value, namespace)) = self.static_attr_at_op(cursor) {
                if (name, namespace) == key {
                    found = Some(value);
                }
                cursor += self.attr_op_len(cursor)?;
            } else {
                break;
            }
        }
        found
    }

    fn root_dynamic_anchor_before(&self, path: TemplatePath) -> Option<&'static TemplateAnchor> {
        self.anchors.iter().find(|anchor| {
            anchor.is_root_level()
                && matches!(
                    anchor.slot_target(),
                    TemplateSlotTarget::BeforeStatic(target) if target == path
                )
        })
    }

    fn trailing_root_dynamic_anchor(&self) -> Option<&'static TemplateAnchor> {
        self.anchors.iter().find(|anchor| {
            anchor.is_root_level()
                && matches!(
                    anchor.slot_target(),
                    TemplateSlotTarget::AppendChildren(path) if path.is_empty()
                )
        })
    }

    /// Iterate template root positions in materialization order.
    pub(crate) fn root_slots(
        &self,
    ) -> impl Iterator<Item = (usize, Option<usize>, Option<&'static TemplateAnchor>)> + '_ {
        let mut op = 0usize;
        let mut static_root_idx = 0usize;
        let mut root_idx = 0usize;
        let mut pending_static = None;
        let mut emitted_trailing_dynamic = false;
        std::iter::from_fn(move || {
            if let Some(static_op) = pending_static.take() {
                let current_root = root_idx;
                root_idx += 1;
                return Some((current_root, Some(static_op), None));
            }

            while op < self.ops.len() && !self.is_static_node_op(op) {
                op = self.next_sibling_op(op);
            }

            if op < self.ops.len() {
                let static_op = op;
                op = self.next_sibling_op(op);
                let static_path = TemplatePath::root(static_root_idx);
                static_root_idx += 1;

                if let Some(anchor) = self.root_dynamic_anchor_before(static_path) {
                    let current_root = root_idx;
                    root_idx += 1;
                    pending_static = Some(static_op);
                    return Some((current_root, None, Some(anchor)));
                }

                let current_root = root_idx;
                root_idx += 1;
                return Some((current_root, Some(static_op), None));
            }

            if !emitted_trailing_dynamic {
                emitted_trailing_dynamic = true;
                if let Some(anchor) = self.trailing_root_dynamic_anchor() {
                    let current_root = root_idx;
                    root_idx += 1;
                    return Some((current_root, None, Some(anchor)));
                }
            }

            None
        })
    }

    /// Return the flat op index immediately after the static node or op at `op`.
    pub(crate) fn next_sibling_op(&self, op: usize) -> usize {
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
    pub(crate) fn is_static_node_op(&self, op: usize) -> bool {
        match self.ops[op].decode() {
            DecodedTemplateOp::Enter { .. } => true,
            DecodedTemplateOp::Text => matches!(
                self.ops.get(op + 1).map(|op| op.decode()),
                Some(DecodedTemplateOp::Static(_))
            ),
            _ => false,
        }
    }

    /// Iterate static child node ops of an element.
    pub(crate) fn static_children(&self, element_op: usize) -> impl Iterator<Item = usize> + '_ {
        let (mut cursor, end) = match self.element_attr_child_ops(element_op) {
            Some((_, child_start, element_end)) => (child_start, element_end),
            None => (0, 0),
        };
        std::iter::from_fn(move || {
            while cursor < end {
                let op = cursor;
                cursor = self.next_sibling_op(cursor);
                if self.is_static_node_op(op) {
                    return Some(op);
                }
            }
            None
        })
    }

    /// Iterate dynamic anchors attached directly to an element.
    pub(crate) fn element_dynamic_anchors(
        &self,
        element_op: usize,
    ) -> impl Iterator<Item = &'static TemplateAnchor> + '_ {
        self.anchors
            .iter()
            .filter(move |anchor| anchor.element_op() == Some(element_op))
    }

    /// Iterate static attributes of an element.
    pub(crate) fn static_attrs(
        &self,
        element_op: usize,
    ) -> impl Iterator<Item = (&'static str, &'static str, Option<&'static str>)> + '_ {
        let (mut cursor, child_start) = match self.element_attr_child_ops(element_op) {
            Some((attr_start, child_start, _)) => (attr_start, child_start),
            None => (0, 0),
        };
        std::iter::from_fn(move || {
            while cursor < child_start {
                let op = cursor;
                cursor += self.attr_op_len(cursor).unwrap_or(1);
                if let Some(attr) = self.static_attr_at_op(op) {
                    return Some(attr);
                }
            }
            None
        })
    }

    const fn compute_dynamic_value_count(anchors: &[TemplateAnchor]) -> u16 {
        let mut max = 0u16;
        let mut i = 0;
        while i < anchors.len() {
            let anchor = anchors[i];
            let end = Self::anchor_value_end(anchor);
            if end > max {
                max = end;
            }
            i += 1;
        }
        max
    }

    const fn anchor_value_end(anchor: TemplateAnchor) -> u16 {
        let end = anchor.value_start as u32 + anchor.value_count as u32;
        if end > u16::MAX as u32 {
            panic!("template dynamic value count exceeds packed anchor capacity");
        }
        end as u16
    }

    /// Compute a content-based hash of template structure.
    /// This is const so it can be used both at compile time and runtime.
    const fn compute_hash(
        ops: &[TemplateOp],
        strings: StaticTemplateStringArray,
        anchors: &[TemplateAnchor],
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
            };
            i += 1;
        }

        // Hash anchor metadata.
        hash = xxh64(&[0xA1], hash);
        let mut i = 0;
        while i < anchors.len() {
            let anchor = anchors[i];
            hash = xxh64(&anchor.op.to_le_bytes(), hash);
            hash = xxh64(&[anchor.kind as u8], hash);
            hash = xxh64(&anchor.path_bits().to_le_bytes(), hash);
            hash = xxh64(&anchor.value_count.to_le_bytes(), hash);
            i += 1;
        }

        hash
    }
}

#[doc(hidden)]
#[allow(missing_docs)]
pub trait TemplateExt {
    fn ops(&self) -> &'static [TemplateOp];

    fn strings(&self) -> &'static [&'static str];

    fn anchors(&self) -> &'static [TemplateAnchor];

    fn dynamic_value_count(&self) -> usize;

    fn root_count(&self) -> usize;

    fn element_meta_at_op(&self, op: usize) -> Option<(&'static str, Option<&'static str>)>;

    fn static_attr_at_op(
        &self,
        op: usize,
    ) -> Option<(&'static str, &'static str, Option<&'static str>)>;

    fn static_text_at_op(&self, op: usize) -> Option<&'static str>;

    fn dynamic_slot_target(&self, idx: usize) -> Option<TemplateSlotTarget>;

    fn root_slots(
        &self,
    ) -> impl Iterator<Item = (usize, Option<usize>, Option<&'static TemplateAnchor>)> + '_;

    fn static_children(&self, element_op: usize) -> impl Iterator<Item = usize> + '_;

    fn element_dynamic_anchors(
        &self,
        element_op: usize,
    ) -> impl Iterator<Item = &'static TemplateAnchor> + '_;

    fn static_attrs(
        &self,
        element_op: usize,
    ) -> impl Iterator<Item = (&'static str, &'static str, Option<&'static str>)> + '_;
}

impl TemplateExt for Template {
    fn ops(&self) -> &'static [TemplateOp] {
        Template::ops(self)
    }

    fn strings(&self) -> &'static [&'static str] {
        Template::strings(self)
    }

    fn anchors(&self) -> &'static [TemplateAnchor] {
        Template::anchors(self)
    }

    fn dynamic_value_count(&self) -> usize {
        Template::dynamic_value_count(self)
    }

    fn root_count(&self) -> usize {
        Template::root_count(self)
    }

    fn element_meta_at_op(&self, op: usize) -> Option<(&'static str, Option<&'static str>)> {
        Template::element_meta_at_op(self, op)
    }

    fn static_attr_at_op(
        &self,
        op: usize,
    ) -> Option<(&'static str, &'static str, Option<&'static str>)> {
        Template::static_attr_at_op(self, op)
    }

    fn static_text_at_op(&self, op: usize) -> Option<&'static str> {
        Template::static_text_at_op(self, op)
    }

    fn dynamic_slot_target(&self, idx: usize) -> Option<TemplateSlotTarget> {
        let anchor = self
            .anchor_for_value(idx)
            .expect("dynamic value index out of range");
        matches!(anchor.kind(), TemplateAnchorKind::Node).then(|| anchor.slot_target())
    }

    fn root_slots(
        &self,
    ) -> impl Iterator<Item = (usize, Option<usize>, Option<&'static TemplateAnchor>)> + '_ {
        Template::root_slots(self)
    }

    fn static_children(&self, element_op: usize) -> impl Iterator<Item = usize> + '_ {
        Template::static_children(self, element_op)
    }

    fn element_dynamic_anchors(
        &self,
        element_op: usize,
    ) -> impl Iterator<Item = &'static TemplateAnchor> + '_ {
        Template::element_dynamic_anchors(self, element_op)
    }

    fn static_attrs(
        &self,
        element_op: usize,
    ) -> impl Iterator<Item = (&'static str, &'static str, Option<&'static str>)> + '_ {
        Template::static_attrs(self, element_op)
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
pub(crate) fn deserialize_strings_leaky<'a, 'de, D>(
    deserializer: D,
) -> Result<&'static [&'static str], D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    let deserialized = Vec::<String>::deserialize(deserializer)?;
    let strings: Vec<&'static str> = deserialized
        .into_iter()
        .map(|string| &*Box::leak(string.into_boxed_str()))
        .collect::<Vec<_>>();
    Ok(&*Box::leak(strings.into_boxed_slice()))
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
