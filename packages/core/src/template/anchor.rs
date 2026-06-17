use super::{TemplatePath, TemplateSlotPath, TemplateSlotTarget};

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

    pub(super) const fn attr(op: u16, path: TemplatePath, value_start: u16, value_count: u16) -> Self {
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

    pub(super) const fn node(op: u16, path: TemplateSlotPath, value_start: u16, value_count: u16) -> Self {
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

    pub(super) const fn single_attr(op: u16, path: TemplatePath, value_start: u16) -> Self {
        Self::attr(op, path, value_start, 1)
    }

    pub(super) const fn single_node(op: u16, path: TemplateSlotPath, value_start: u16) -> Self {
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

    pub(super) const fn kind(self) -> TemplateAnchorKind {
        self.kind
    }

    pub(super) const fn path_bits(self) -> u128 {
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

    pub(super) const fn same_slot_bits(self, op: u16, kind: TemplateAnchorKind, path: u128) -> bool {
        self.op == op
            && matches!(
                (self.kind, kind),
                (TemplateAnchorKind::Attr, TemplateAnchorKind::Attr)
                    | (TemplateAnchorKind::Node, TemplateAnchorKind::Node)
            )
            && self.path == path
    }

    pub(super) const fn should_fill_before(self, other: Self) -> bool {
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
