use super::{TemplatePath, TemplateSlotPath, TemplateSlotTarget};

/// Sentinel `parent_op_index` value marking a [`TemplateAnchor`] for a root-level dynamic node slot,
/// which has no enclosing static element.
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
    /// One past the last dynamic value index owned by this anchor.
    pub(crate) value_end: u16,
}

impl TemplateAnchor {
    pub fn parent_element_op_index(self) -> Option<usize> {
        (self.parent_op_index != ROOT_PARENT_OP_INDEX).then_some(self.parent_op_index as usize)
    }

    pub(crate) const fn slot_path(self) -> TemplateSlotPath {
        TemplateSlotPath::from_bits(self.path)
    }

    pub const fn slot_target(self) -> TemplateSlotTarget {
        self.slot_path().target()
    }

    pub const fn static_path(self) -> TemplatePath {
        self.slot_path().static_parent()
    }

    pub const fn values(self) -> std::ops::Range<usize> {
        self.value_start as usize..self.value_end as usize
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
