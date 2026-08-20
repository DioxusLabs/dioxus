use super::{TemplatePath, TemplateSlotPath, TemplateSlotTarget};

/// Sentinel `parent_op_index` value marking a [`TemplateAnchor`] for a root-level dynamic node slot,
/// which has no enclosing static element.
pub(crate) const ROOT_PARENT_OP_INDEX: u16 = u16::MAX;

/// A dynamic slot anchor in a static template.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct TemplateAnchor {
    /// Dynamic slot path.
    pub(crate) path: TemplateSlotPath,
    /// Static template operation index for the anchor's parent element.
    pub(crate) parent_op_index: u16,
    /// First dynamic node index owned by this anchor.
    pub(crate) node_start: u16,
    /// One past the last dynamic node index owned by this anchor.
    pub(crate) node_end: u16,
    /// First dynamic attribute index owned by this anchor.
    pub(crate) attr_start: u16,
    /// One past the last dynamic attribute index owned by this anchor.
    pub(crate) attr_end: u16,
}

impl TemplateAnchor {
    pub fn parent_element_op_index(self) -> Option<usize> {
        (self.parent_op_index != ROOT_PARENT_OP_INDEX).then_some(self.parent_op_index as usize)
    }

    pub(crate) const fn slot_path(self) -> TemplateSlotPath {
        self.path
    }

    pub const fn slot_target(self) -> TemplateSlotTarget {
        self.slot_path().target()
    }

    pub const fn static_path(self) -> TemplatePath {
        self.slot_path().static_path()
    }

    pub const fn is_last_static_node(self) -> bool {
        self.slot_path().is_last_static_node()
    }

    pub const fn is_parent_append_target(self) -> bool {
        self.is_last_static_node() && self.parent_op_index != ROOT_PARENT_OP_INDEX
    }

    pub const fn nodes(self) -> std::ops::Range<usize> {
        self.node_start as usize..self.node_end as usize
    }

    pub const fn attributes(self) -> std::ops::Range<usize> {
        self.attr_start as usize..self.attr_end as usize
    }

    pub(crate) const fn same_anchor(self, parent_op_index: u16, path: TemplateSlotPath) -> bool {
        self.parent_op_index == parent_op_index && self.path.bits() == path.bits()
    }
}
