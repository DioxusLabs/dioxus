use super::{DecodedTemplateOp, TemplateAnchor};
use crate::TemplateSlotTarget;
use crate::op::TemplateOp;

/// A static layout of a UI tree.
///
/// Templates describe the stable parts of a view while runtime values provide
/// the dynamic nodes and attributes for each render.
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
#[derive(Clone, Copy, Eq)]
pub struct Template {
    /// Flat static template operations.
    #[cfg_attr(
        feature = "serialize",
        serde(deserialize_with = "super::serialization::deserialize_leaky")
    )]
    ops: &'static [TemplateOp],

    /// Static strings referenced by static string operations.
    #[cfg_attr(
        feature = "serialize",
        serde(deserialize_with = "super::serialization::deserialize_strings_leaky")
    )]
    strings: &'static [&'static str],

    /// Dynamic node and attribute ranges in document order, each anchored to a static element.
    #[cfg_attr(
        feature = "serialize",
        serde(deserialize_with = "super::serialization::deserialize_leaky")
    )]
    anchors: &'static [TemplateAnchor],

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

/// A static element or text node inside a [`Template`].
#[derive(Clone, Copy)]
pub enum StaticTemplateNode<'a> {
    /// A static template element.
    Element(StaticTemplateElement<'a>),
    /// A static template text node.
    Text(StaticTemplateText<'a>),
}

impl<'a> StaticTemplateNode<'a> {
    /// Return the flat template op that starts this node.
    pub fn op(self) -> usize {
        match self {
            Self::Element(element) => element.op(),
            Self::Text(text) => text.op(),
        }
    }

    /// Return this node as an element, if it is one.
    pub fn as_element(self) -> Option<StaticTemplateElement<'a>> {
        match self {
            Self::Element(element) => Some(element),
            Self::Text(_) => None,
        }
    }

    /// Return this node as text, if it is one.
    pub fn as_text(self) -> Option<StaticTemplateText<'a>> {
        match self {
            Self::Element(_) => None,
            Self::Text(text) => Some(text),
        }
    }
}

/// A static element inside a [`Template`].
#[derive(Clone, Copy)]
pub struct StaticTemplateElement<'a> {
    template: &'a Template,
    op: usize,
}

impl<'a> StaticTemplateElement<'a> {
    /// Return the flat template op that starts this element.
    pub fn op(self) -> usize {
        self.op
    }

    /// Return the element tag.
    pub fn tag(self) -> &'static str {
        self.template
            .element_meta_at_op(self.op)
            .expect("static element")
            .0
    }

    /// Return the element namespace.
    pub fn namespace(self) -> Option<&'static str> {
        self.template
            .element_meta_at_op(self.op)
            .expect("static element")
            .1
    }

    /// Iterate static attributes on this element.
    pub fn attributes(self) -> StaticTemplateAttributeIter<'a> {
        let (cursor, end, _) = self
            .template
            .element_attr_child_ops(self.op)
            .expect("static element");
        StaticTemplateAttributeIter {
            template: self.template,
            cursor,
            end,
        }
    }

    /// Iterate static child nodes of this element.
    pub fn children(self) -> StaticTemplateNodeIter<'a> {
        let (_, cursor, end) = self
            .template
            .element_attr_child_ops(self.op)
            .expect("static element");
        StaticTemplateNodeIter {
            template: self.template,
            cursor,
            end,
        }
    }

    /// Find a static attr fallback value for a key in this element.
    pub fn attribute_value(
        self,
        key: (&'static str, Option<&'static str>),
    ) -> Option<&'static str> {
        self.attributes()
            .find(|attr| (attr.name, attr.namespace) == key)
            .map(|attr| attr.value)
    }
}

/// A static text node inside a [`Template`].
#[derive(Clone, Copy)]
pub struct StaticTemplateText<'a> {
    template: &'a Template,
    op: usize,
}

impl StaticTemplateText<'_> {
    /// Return the flat template op that starts this text node.
    pub fn op(self) -> usize {
        self.op
    }

    /// Return the text value.
    pub fn text(self) -> &'static str {
        self.template
            .static_text_at_op(self.op)
            .expect("static text")
    }
}

/// A static attribute on a template element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StaticTemplateAttribute {
    /// Attribute name.
    pub name: &'static str,
    /// Attribute value.
    pub value: &'static str,
    /// Attribute namespace.
    pub namespace: Option<&'static str>,
}

/// Iterator over static template nodes.
#[derive(Clone, Copy)]
pub struct StaticTemplateNodeIter<'a> {
    template: &'a Template,
    cursor: usize,
    end: usize,
}

impl<'a> Iterator for StaticTemplateNodeIter<'a> {
    type Item = StaticTemplateNode<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.cursor < self.end {
            let op = self.cursor;
            self.cursor = self.template.next_sibling_op(op);
            if let Some(node) = self.template.static_node(op) {
                return Some(node);
            }
        }
        None
    }
}

/// Iterator over static template attributes.
#[derive(Clone, Copy)]
pub struct StaticTemplateAttributeIter<'a> {
    template: &'a Template,
    cursor: usize,
    end: usize,
}

impl Iterator for StaticTemplateAttributeIter<'_> {
    type Item = StaticTemplateAttribute;

    fn next(&mut self) -> Option<Self::Item> {
        while self.cursor < self.end {
            let op = self.cursor;
            self.cursor += self.template.attr_op_len(op).unwrap_or(1);
            if let Some((name, value, namespace)) = self.template.static_attr_at_op(op) {
                return Some(StaticTemplateAttribute {
                    name,
                    value,
                    namespace,
                });
            }
        }
        None
    }
}

impl std::fmt::Debug for Template {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Template").finish_non_exhaustive()
    }
}

impl Template {
    /// Create a new template.
    pub(crate) const fn new(
        ops: &'static [TemplateOp],
        strings: &'static [&'static str],
        anchors: &'static [TemplateAnchor],
    ) -> Self {
        Self {
            ops,
            strings,
            anchors,
            hash: Self::compute_hash(ops, strings, anchors),
        }
    }

    /// Get the template static string pool.
    pub const fn strings(&self) -> &'static [&'static str] {
        self.strings
    }

    /// Iterate decoded template operations.
    pub fn decoded_ops(&self) -> impl ExactSizeIterator<Item = DecodedTemplateOp> + '_ {
        self.ops.iter().map(|op| op.decode())
    }

    /// Get dynamic slot anchors in document order.
    pub const fn anchors(&self) -> &'static [TemplateAnchor] {
        self.anchors
    }

    /// Iterate static root nodes in this template.
    pub fn static_roots(&self) -> StaticTemplateNodeIter<'_> {
        StaticTemplateNodeIter {
            template: self,
            cursor: 0,
            end: self.ops.len(),
        }
    }

    /// Return a static node by flat template op.
    pub fn static_node(&self, op: usize) -> Option<StaticTemplateNode<'_>> {
        self.static_element(op)
            .map(StaticTemplateNode::Element)
            .or_else(|| self.static_text(op).map(StaticTemplateNode::Text))
    }

    /// Return a static element by flat template op.
    pub fn static_element(&self, op: usize) -> Option<StaticTemplateElement<'_>> {
        self.element_meta_at_op(op)
            .is_some()
            .then_some(StaticTemplateElement { template: self, op })
    }

    /// Return a static text node by flat template op.
    pub fn static_text(&self, op: usize) -> Option<StaticTemplateText<'_>> {
        self.static_text_at_op(op)
            .is_some()
            .then_some(StaticTemplateText { template: self, op })
    }

    /// Get a static string from this template's string pool.
    fn string(&self, id: u16) -> &'static str {
        self.strings[id as usize]
    }

    /// Decode an element op into its subtree length and namespace presence.
    fn enter_meta(&self, op: usize) -> Option<(usize, bool)> {
        match self.ops.get(op).map(|op| op.decode()) {
            Some(DecodedTemplateOp::Enter { skip, namespace }) => Some((skip as usize, namespace)),
            _ => None,
        }
    }

    /// Return the static string referenced by an op.
    fn static_string_at_op(&self, op: usize) -> Option<&'static str> {
        match self.ops.get(op).map(|op| op.decode()) {
            Some(DecodedTemplateOp::Static(id)) => Some(self.string(id)),
            _ => None,
        }
    }

    fn element_meta_at_op(&self, op: usize) -> Option<(&'static str, Option<&'static str>)> {
        let (_, has_namespace) = self.enter_meta(op)?;
        let tag = self.static_string_at_op(op + 1)?;
        let namespace = has_namespace
            .then(|| self.static_string_at_op(op + 2))
            .flatten();
        Some((tag, namespace))
    }

    fn element_children_start(&self, op: usize) -> Option<usize> {
        let (_, has_namespace) = self.enter_meta(op)?;
        Some(op + if has_namespace { 3 } else { 2 })
    }

    fn static_attr_at_op(
        &self,
        op: usize,
    ) -> Option<(&'static str, &'static str, Option<&'static str>)> {
        let namespace = match self.ops.get(op).map(|op| op.decode()) {
            Some(DecodedTemplateOp::Attr { namespace }) => namespace,
            _ => return None,
        };
        let name = self.static_string_at_op(op + 1)?;
        let value = self.static_string_at_op(op + 2)?;
        let namespace = namespace
            .then(|| self.static_string_at_op(op + 3))
            .flatten();
        Some((name, value, namespace))
    }

    fn static_text_at_op(&self, op: usize) -> Option<&'static str> {
        (self.ops.get(op).map(|op| op.decode()) == Some(DecodedTemplateOp::Text))
            .then(|| self.static_string_at_op(op + 1))
            .flatten()
    }

    fn attr_op_len(&self, op: usize) -> Option<usize> {
        match self.ops.get(op).map(|op| op.decode()) {
            Some(DecodedTemplateOp::Attr { namespace: true }) => Some(4),
            Some(DecodedTemplateOp::Attr { .. }) => Some(3),
            _ => None,
        }
    }

    fn element_end(&self, op: usize) -> Option<usize> {
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

    /// Return the number of materialized root positions.
    pub fn root_position_count(&self) -> usize {
        self.static_root_count() + self.root_level_dynamic_anchor_count()
    }

    /// Map a static root index to the materialized root position that renders it.
    pub fn root_position_for_static_root(&self, static_root_idx: usize) -> Option<usize> {
        (static_root_idx < self.static_root_count())
            .then(|| static_root_idx + self.root_dynamic_before_static_count(static_root_idx, true))
    }

    /// Return the materialized root position that owns an anchor.
    pub fn root_position_for_anchor(&self, anchor_idx: usize) -> Option<usize> {
        let anchor = self.anchors.get(anchor_idx)?;
        match anchor.slot_target() {
            TemplateSlotTarget::BeforeStatic(path) if path.is_root() => {
                let static_root_idx = path.segment(0) as usize;
                Some(
                    static_root_idx + self.root_dynamic_before_static_count(static_root_idx, false),
                )
            }
            TemplateSlotTarget::AppendChildren(path) if path.is_empty() => Some(
                self.static_root_count() + self.root_dynamic_before_static_count(usize::MAX, true),
            ),
            TemplateSlotTarget::BeforeStatic(path) => {
                self.root_position_for_static_root(path.segment(0) as usize)
            }
            TemplateSlotTarget::AppendChildren(path) => (!path.is_empty())
                .then(|| path.segment(0) as usize)
                .and_then(|static_root_idx| self.root_position_for_static_root(static_root_idx)),
        }
    }

    fn static_root_count(&self) -> usize {
        let mut op = 0usize;
        let mut count = 0usize;
        while op < self.ops.len() {
            if self.is_static_node_op(op) {
                count += 1;
            }
            op = self.next_sibling_op(op);
        }
        count
    }

    fn root_level_dynamic_anchor_count(&self) -> usize {
        self.anchors
            .iter()
            .filter(|anchor| anchor.parent_element_op_index().is_none())
            .filter(|anchor| match anchor.slot_target() {
                TemplateSlotTarget::BeforeStatic(path) => path.is_root(),
                TemplateSlotTarget::AppendChildren(path) => path.is_empty(),
            })
            .count()
    }

    fn root_dynamic_before_static_count(
        &self,
        static_root_idx: usize,
        include_current: bool,
    ) -> usize {
        self.anchors
            .iter()
            .filter(|anchor| anchor.parent_element_op_index().is_none())
            .filter_map(|anchor| match anchor.slot_target() {
                TemplateSlotTarget::BeforeStatic(path) if path.is_root() => {
                    Some(path.segment(0) as usize)
                }
                _ => None,
            })
            .filter(|&idx| {
                if include_current {
                    idx <= static_root_idx
                } else {
                    idx < static_root_idx
                }
            })
            .count()
    }

    fn next_sibling_op(&self, op: usize) -> usize {
        Self::next_sibling_op_in(self.ops, op)
    }

    /// Return true if an op starts an element or static text node.
    fn is_static_node_op(&self, op: usize) -> bool {
        Self::is_static_node_op_in(self.ops, op)
    }

    const fn is_static_node_op_in(ops: &[TemplateOp], op: usize) -> bool {
        match ops[op].decode() {
            DecodedTemplateOp::Enter { .. } => true,
            DecodedTemplateOp::Text => {
                op + 1 < ops.len() && matches!(ops[op + 1].decode(), DecodedTemplateOp::Static(_))
            }
            _ => false,
        }
    }

    const fn next_sibling_op_in(ops: &[TemplateOp], op: usize) -> usize {
        match ops[op].decode() {
            DecodedTemplateOp::Enter { skip, .. } => op + skip as usize,
            DecodedTemplateOp::Text => op + 2,
            DecodedTemplateOp::Attr { namespace: true } => op + 4,
            DecodedTemplateOp::Attr { .. } => op + 3,
            _ => op + 1,
        }
    }

    /// Compute a content-based hash of template structure.
    /// This is const so it can be used both at compile time and runtime.
    const fn compute_hash(
        ops: &[TemplateOp],
        strings: &'static [&'static str],
        anchors: &[TemplateAnchor],
    ) -> u64 {
        use xxhash_rust::const_xxh64::xxh64;

        let mut hash = 0u64;

        // Raw operations
        let mut i = 0;
        while i < ops.len() {
            hash = xxh64(&ops[i].bits().to_le_bytes(), hash);
            i += 1;
        }

        // Static strings
        hash = xxh64(&[0xA0], hash);
        let mut i = 0;
        while i < strings.len() {
            let string = strings[i];
            let bytes = string.as_bytes();
            hash = xxh64(bytes, hash);
            i += 1;
        }

        // Hash anchor metadata.
        hash = xxh64(&[0xA1], hash);
        let mut i = 0;
        while i < anchors.len() {
            let anchor = anchors[i];
            hash = xxh64(&anchor.parent_op_index.to_le_bytes(), hash);
            hash = xxh64(&anchor.slot_path().bits().to_le_bytes(), hash);
            hash = xxh64(&anchor.node_start.to_le_bytes(), hash);
            hash = xxh64(&anchor.node_end.to_le_bytes(), hash);
            hash = xxh64(&anchor.attr_start.to_le_bytes(), hash);
            hash = xxh64(&anchor.attr_end.to_le_bytes(), hash);
            i += 1;
        }

        hash
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for Template {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct SerializedTemplate {
            #[serde(deserialize_with = "super::serialization::deserialize_leaky")]
            ops: &'static [TemplateOp],
            #[serde(deserialize_with = "super::serialization::deserialize_strings_leaky")]
            strings: &'static [&'static str],
            #[serde(deserialize_with = "super::serialization::deserialize_leaky")]
            anchors: &'static [TemplateAnchor],
            hash: u64,
        }

        let serialized = SerializedTemplate::deserialize(deserializer)?;
        // Trust the serialized hash that the original builder computed.
        Ok(Self {
            ops: serialized.ops,
            strings: serialized.strings,
            anchors: serialized.anchors,
            hash: serialized.hash,
        })
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

impl PartialOrd for Template {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Template {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.hash.cmp(&other.hash)
    }
}
