use super::{DecodedTemplateAttrNamespace, DecodedTemplateOp, TemplateAnchor, TemplatePath};
use crate::TemplateSlotTarget;
use crate::op::TemplateOp;

type StaticTemplateOpArray = &'static [TemplateOp];
type StaticTemplateStringArray = &'static [&'static str];

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
    ops: StaticTemplateOpArray,

    /// Static strings referenced by static string operations.
    #[cfg_attr(
        feature = "serialize",
        serde(deserialize_with = "super::serialization::deserialize_strings_leaky")
    )]
    strings: StaticTemplateStringArray,

    /// Dynamic value groups in reverse breadth-first fill order, each anchored to a static element.
    #[cfg_attr(
        feature = "serialize",
        serde(deserialize_with = "super::serialization::deserialize_leaky")
    )]
    anchors: &'static [TemplateAnchor],

    /// Total number of runtime dynamic values this template expects.
    #[cfg_attr(feature = "serialize", serde(skip))]
    dynamic_value_count: u16,

    /// Total number of root positions in this template.
    #[cfg_attr(feature = "serialize", serde(skip))]
    root_count: u16,

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

impl std::fmt::Debug for Template {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Template").finish_non_exhaustive()
    }
}

impl Template {
    /// Create a new template.
    ///
    /// `value_kind_hash` folds in the per-dynamic-value kind (attribute vs node)
    /// in fill order. Attributes and nodes share kind-agnostic anchors — the
    /// runtime value decides which a slot is — so two templates with the same op
    /// tape and anchors but a different kind layout (`{attr}` where the other has
    /// `{node}`) must not compare equal. Folding the kind layout into the hash
    /// keeps `Template` equality meaning "structurally interchangeable for
    /// diffing" without storing the kinds anywhere on the template.
    pub(crate) const fn new(
        ops: &'static [TemplateOp],
        strings: StaticTemplateStringArray,
        anchors: &'static [TemplateAnchor],
        value_kind_hash: u64,
    ) -> Self {
        Self::validate_anchors(anchors);
        Self {
            ops,
            strings,
            anchors,
            dynamic_value_count: Self::compute_dynamic_value_count(anchors),
            root_count: Self::compute_root_count(ops, anchors),
            hash: Self::compute_hash(ops, strings, anchors, value_kind_hash),
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

    const fn validate_anchors(anchors: &[TemplateAnchor]) {
        let mut index = 0;
        let mut has_start = anchors.is_empty();
        while index < anchors.len() {
            let anchor = anchors[index];
            if anchor.value_count == 0 {
                panic!("bad anchor");
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
                        panic!("anchor overlap");
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
                    panic!("anchor gap");
                }
            }

            index += 1;
        }

        if !has_start {
            panic!("anchor start");
        }
    }

    /// Get dynamic value anchors in native fill order.
    pub const fn anchors(&self) -> &'static [TemplateAnchor] {
        self.anchors
    }

    pub fn anchors_in_document_order(
        &self,
    ) -> impl DoubleEndedIterator<Item = &'static TemplateAnchor> + '_ {
        (0..self.dynamic_value_count()).filter_map(move |idx| {
            self.anchors
                .iter()
                .find(|anchor| anchor.value_start as usize == idx)
        })
    }

    /// Return the total number of dynamic values.
    pub fn dynamic_value_count(&self) -> usize {
        self.dynamic_value_count as usize
    }

    pub fn anchor_for_value(&self, idx: usize) -> Option<&'static TemplateAnchor> {
        self.anchors.iter().find(|a| a.values().contains(&idx))
    }

    /// Get the number of root positions in this template.
    pub fn root_count(&self) -> usize {
        self.root_count as usize
    }

    /// Get a static string from this template's string pool.
    pub fn string(&self, id: u16) -> &'static str {
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

    pub fn first_child_node_op(&self, element_op: usize) -> Option<usize> {
        Some(self.element_attr_child_ops(element_op)?.1)
    }

    /// Find a static attr fallback value for a key in an element.
    pub fn static_attr_value_for_key(
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
            anchor.parent_element_op_index().is_none()
                && matches!(
                    anchor.slot_target(),
                    TemplateSlotTarget::BeforeStatic(target) if target == path
                )
        })
    }

    fn trailing_root_dynamic_anchor(&self) -> Option<&'static TemplateAnchor> {
        self.anchors.iter().find(|anchor| {
            anchor.parent_element_op_index().is_none()
                && matches!(
                    anchor.slot_target(),
                    TemplateSlotTarget::AppendChildren(path) if path.is_empty()
                )
        })
    }

    /// Iterate template root positions in materialization order.
    pub fn root_slots(
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

    /// Map a static root index to the materialization root index that renders it.
    ///
    /// `static_root_idx` counts only static root nodes, while the returned index counts every
    /// materialization root (static nodes and root-level dynamic anchors alike).
    pub fn materialization_root_for_static(&self, static_root_idx: usize) -> Option<usize> {
        let mut current_static_root = 0;
        for (root_idx, static_op, _) in self.root_slots() {
            if static_op.is_none() {
                continue;
            }
            if current_static_root == static_root_idx {
                return Some(root_idx);
            }
            current_static_root += 1;
        }
        None
    }

    /// Return the flat op index immediately after the static node or op at `op`.
    pub fn next_sibling_op(&self, op: usize) -> usize {
        Self::next_sibling_op_in(self.ops, op)
    }

    /// Return true if an op starts an element or static text node.
    pub fn is_static_node_op(&self, op: usize) -> bool {
        Self::is_static_node_op_in(self.ops, op)
    }

    /// Iterate static child node ops of an element.
    pub fn static_children(&self, element_op: usize) -> impl Iterator<Item = usize> + '_ {
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
    pub fn element_dynamic_anchors(
        &self,
        element_op: usize,
    ) -> impl Iterator<Item = &'static TemplateAnchor> + '_ {
        self.anchors
            .iter()
            .filter(move |anchor| anchor.parent_element_op_index() == Some(element_op))
    }

    /// Iterate static attributes of an element.
    pub fn static_attrs(
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

    const fn compute_root_count(ops: &[TemplateOp], anchors: &[TemplateAnchor]) -> u16 {
        let mut count = 0u16;
        let mut op = 0usize;
        while op < ops.len() {
            if Self::is_static_node_op_in(ops, op) {
                count += 1;
            }
            op = Self::next_sibling_op_in(ops, op);
        }

        let mut i = 0usize;
        while i < anchors.len() {
            if anchors[i].parent_op_index == super::anchor::ROOT_PARENT_OP_INDEX {
                count += 1;
            }
            i += 1;
        }

        count
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
            DecodedTemplateOp::Attr {
                namespace: DecodedTemplateAttrNamespace::Custom,
            } => op + 4,
            DecodedTemplateOp::Attr { .. } => op + 3,
            _ => op + 1,
        }
    }

    const fn anchor_value_end(anchor: TemplateAnchor) -> u16 {
        let end = anchor.value_start as u32 + anchor.value_count as u32;
        if end > u16::MAX as u32 {
            panic!("anchor overflow");
        }
        end as u16
    }

    /// Compute a content-based hash of template structure.
    /// This is const so it can be used both at compile time and runtime.
    const fn compute_hash(
        ops: &[TemplateOp],
        strings: StaticTemplateStringArray,
        anchors: &[TemplateAnchor],
        value_kind_hash: u64,
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
            hash = xxh64(&anchor.parent_op_index.to_le_bytes(), hash);
            hash = xxh64(&anchor.path.to_le_bytes(), hash);
            hash = xxh64(&anchor.value_start.to_le_bytes(), hash);
            hash = xxh64(&anchor.value_count.to_le_bytes(), hash);
            i += 1;
        }

        // Fold the per-value kind layout (attribute vs node) so structurally
        // incompatible templates that share an op tape and anchors hash apart.
        hash = xxh64(&[0xA2], hash);
        hash = xxh64(&value_kind_hash.to_le_bytes(), hash);

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
            strings: StaticTemplateStringArray,
            #[serde(deserialize_with = "super::serialization::deserialize_leaky")]
            anchors: &'static [TemplateAnchor],
            hash: u64,
        }

        let serialized = SerializedTemplate::deserialize(deserializer)?;
        // The hash folds in the per-value kind layout, which is not recoverable
        // from the op tape and anchors alone, so trust the serialized hash that
        // the original builder computed rather than recomputing it here.
        Self::validate_anchors(serialized.anchors);
        Ok(Self {
            ops: serialized.ops,
            strings: serialized.strings,
            anchors: serialized.anchors,
            dynamic_value_count: Self::compute_dynamic_value_count(serialized.anchors),
            root_count: Self::compute_root_count(serialized.ops, serialized.anchors),
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
