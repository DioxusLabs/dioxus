use super::{TemplateOp, TemplatePath, TemplateSlotPath, TemplateSlotTarget};

/// Sentinel `parent_op_index` value marking a [`TemplateAnchor`] for a root-level dynamic node slot,
/// which has no enclosing static element.
#[doc(hidden)]
pub(crate) const ROOT_PARENT_OP_INDEX: u16 = u16::MAX;

/// A dynamic value anchor in a static template.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct TemplateAnchor {
    /// Encoded dynamic slot path.
    pub(crate) path: u128,
    /// Static template operation index for the anchor's parent element.
    pub(crate) parent_op_index: u16,
    /// First dynamic value index owned by this anchor.
    pub(crate) value_start: u16,
    /// Number of dynamic values owned by this anchor.
    pub(crate) value_count: u16,
}

impl TemplateAnchor {
    /// Create an anchor from typed template coordinates.
    pub const fn new(
        parent_op: Option<usize>,
        path: TemplateSlotPath,
        values: std::ops::Range<usize>,
    ) -> Self {
        let parent_op_index = Self::parent_op_index(parent_op);
        let (value_start, value_count) = Self::range_parts(values);
        Self {
            path: path.bits(),
            parent_op_index,
            value_start,
            value_count,
        }
    }

    const fn parent_op_index(parent_op: Option<usize>) -> u16 {
        match parent_op {
            Some(parent_op) => {
                if parent_op >= TemplateOp::MAX_CAP {
                    panic!("anchor parent op exceeds packed op capacity");
                }
                parent_op as u16
            }
            None => ROOT_PARENT_OP_INDEX,
        }
    }

    const fn range_parts(values: std::ops::Range<usize>) -> (u16, u16) {
        if values.start >= values.end {
            panic!("bad anchor");
        }
        if values.end > u16::MAX as usize {
            panic!("anchor overflow");
        }
        (values.start as u16, (values.end - values.start) as u16)
    }

    pub fn parent_element_op_index(self) -> Option<usize> {
        (self.parent_op_index != ROOT_PARENT_OP_INDEX).then_some(self.parent_op_index as usize)
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

    pub fn values(self) -> std::ops::Range<usize> {
        self.value_start as usize..(self.value_start as usize + self.value_count as usize)
    }

    pub(crate) const fn same_anchor(self, parent_op_index: u16, path: u128) -> bool {
        self.parent_op_index == parent_op_index && self.path == path
    }

    pub(crate) const fn should_fill_before(self, other: Self) -> bool {
        let self_depth = self.slot_path().fill_depth();
        let other_depth = other.slot_path().fill_depth();
        if self_depth != other_depth {
            return self_depth > other_depth;
        }

        self.value_start > other.value_start
    }
}
