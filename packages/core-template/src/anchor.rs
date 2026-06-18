use super::{TemplatePath, TemplateSlotPath, TemplateSlotTarget};

/// Sentinel `op` value marking a [`TemplateAnchor`] for a root-level dynamic node slot, which has no
/// enclosing static element.
#[doc(hidden)]
pub const ROOT_ANCHOR_OP: u16 = u16::MAX;

#[doc(hidden)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct TemplateAnchor {
    pub path: u128,
    pub op: u16,
    pub value_start: u16,
    pub value_count: u16,
}

impl TemplateAnchor {
    pub const fn from_raw_parts(op: u16, path: u128, values: std::ops::Range<u16>) -> Self {
        let (value_start, value_count) = Self::range_parts(values);
        Self {
            path,
            op,
            value_start,
            value_count,
        }
    }

    const fn range_parts(values: std::ops::Range<u16>) -> (u16, u16) {
        if values.start >= values.end {
            panic!("bad anchor");
        }
        (values.start, values.end - values.start)
    }

    const fn single_value_range(value_start: u16) -> std::ops::Range<u16> {
        if value_start == u16::MAX {
            panic!("anchor overflow");
        }
        value_start..value_start + 1
    }

    pub const fn root_node(value_index: u16, root_idx: usize, appends: bool) -> Self {
        let slot = if appends {
            TemplateSlotPath::append_children(TemplatePath::empty())
        } else {
            TemplateSlotPath::before_static(TemplatePath::root(root_idx))
        };
        Self::from_raw_parts(
            ROOT_ANCHOR_OP,
            slot.bits(),
            Self::single_value_range(value_index),
        )
    }

    pub const fn path_bits(self) -> u128 {
        self.path
    }

    pub fn element_op(self) -> Option<usize> {
        (self.op != ROOT_ANCHOR_OP).then_some(self.op as usize)
    }

    pub fn is_root_level(self) -> bool {
        self.op == ROOT_ANCHOR_OP
    }

    pub const fn slot_path(self) -> TemplateSlotPath {
        TemplateSlotPath::from_bits(self.path)
    }

    pub const fn slot_target(self) -> TemplateSlotTarget {
        self.slot_path().target()
    }

    pub const fn static_path(self) -> TemplatePath {
        self.slot_path().static_parent()
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

    pub const fn same_anchor(self, op: u16, path: u128) -> bool {
        self.op == op && self.path == path
    }

    pub const fn should_fill_before(self, other: Self) -> bool {
        let self_depth = self.slot_path().fill_depth();
        let other_depth = other.slot_path().fill_depth();
        if self_depth != other_depth {
            return self_depth > other_depth;
        }

        self.value_start > other.value_start
    }
}
