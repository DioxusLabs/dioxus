use std::num::NonZeroU128;

/// A compact path from a template root to a static node or dynamic attribute.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TemplatePath {
    path: u128,
}

impl TemplatePath {
    /// Create an empty path.
    pub const fn empty() -> Self {
        Self { path: 0 }
    }

    /// Create a path from compact path bits.
    pub const fn from_bits(path: u128) -> Self {
        Self { path }
    }

    /// Return the path for a root position.
    pub const fn root(index: usize) -> Self {
        if index >= u128::BITS as usize {
            return Self::empty();
        }

        Self {
            path: 1u128 << index,
        }
    }

    /// Return the compact path bits.
    pub const fn bits(self) -> u128 {
        self.path
    }

    /// Return the path for the first child of this path.
    pub(crate) const fn next_child(self) -> Self {
        Self {
            path: (self.path << 1) | 1,
        }
    }

    /// Return the path for the next sibling of this path.
    pub(crate) const fn next_sibling(self) -> Self {
        Self {
            path: self.path << 1,
        }
    }

    /// Return the parent path.
    pub(crate) const fn parent(self) -> Self {
        Self {
            path: self.path >> 1,
        }
    }

    /// Split a path to a static node into its parent path and the node's index among that
    /// parent's children.
    pub const fn split_insertion(self) -> (TemplatePath, usize) {
        if self.path == 0 {
            return (TemplatePath::from_bits(0), 0);
        }

        let insertion_index = self.path.trailing_zeros() as usize;
        let parent = (self.path >> insertion_index) >> 1;
        (TemplatePath::from_bits(parent), insertion_index)
    }

    /// Return true if this path points at a template root node (a direct child of the root).
    pub const fn is_root(self) -> bool {
        self.len() == 1
    }

    /// Return the number of path segments.
    pub const fn len(self) -> usize {
        self.path.count_ones() as usize
    }

    /// Return true if this path has no segments.
    pub const fn is_empty(self) -> bool {
        self.path == 0
    }

    /// Return the path segment at `index`.
    pub fn segment(self, index: usize) -> u8 {
        let mut path = self.path;
        let mut remaining_segments = index;

        loop {
            let bit_len = u128::BITS - path.leading_zeros();
            if bit_len == 0 {
                panic!("bad path segment");
            }

            let marker = 1u128 << (bit_len - 1);
            let remaining_path = path ^ marker;
            if remaining_segments == 0 {
                let next_marker_bit_len = u128::BITS - remaining_path.leading_zeros();
                return if next_marker_bit_len == 0 {
                    (bit_len - 1) as u8
                } else {
                    (bit_len - next_marker_bit_len - 1) as u8
                };
            }

            path = remaining_path;
            remaining_segments -= 1;
        }
    }

    /// Return true if this compact path starts with `ancestor`.
    pub fn starts_with(self, ancestor: TemplatePath) -> bool {
        let ancestor_len = ancestor.len();
        if ancestor_len == 0 {
            return true;
        }

        if ancestor_len > self.len() {
            return false;
        }

        if ancestor_len == self.len() {
            return self.path == ancestor.path;
        }

        let suffix_bits = self.bit_len() - ancestor.bit_len();
        suffix_bits > 0
            && (self.path >> suffix_bits) == ancestor.path
            && ((self.path >> (suffix_bits - 1)) & 1) == 1
    }

    /// Return the number of raw child/sibling bits in this path.
    pub(crate) fn bit_len(self) -> u32 {
        u128::BITS - self.path.leading_zeros()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct TemplateSlotPath(NonZeroU128);

/// The resolved renderer target for a dynamic node slot.
#[derive(Clone, Copy, PartialEq, Eq)]
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
            None => panic!("bad slot path"),
        }
    }

    const fn encode_payload(path: TemplatePath) -> u128 {
        let payload = path.bits();
        if payload > Self::MAX_PAYLOAD {
            panic!("slot path overflow");
        }
        payload << 1
    }

    pub(crate) const fn before_static(path: TemplatePath) -> Self {
        if path.is_empty() {
            panic!("bad slot target");
        }
        Self::new(Self::encode_payload(path))
    }

    pub(crate) const fn append_children(path: TemplatePath) -> Self {
        Self::new(Self::encode_payload(path) | Self::TARGET_APPEND_CHILDREN)
    }

    /// Create a slot path from raw bits.
    pub(crate) const fn from_bits(bits: u128) -> Self {
        Self::new(bits)
    }

    /// Return the raw tagged bits.
    pub(crate) const fn bits(self) -> u128 {
        self.0.get()
    }

    pub(crate) const fn target(self) -> TemplateSlotTarget {
        let bits = self.bits();
        let path = TemplatePath::from_bits(bits >> 1);
        if bits & Self::TARGET_APPEND_CHILDREN == Self::TARGET_APPEND_CHILDREN {
            TemplateSlotTarget::AppendChildren(path)
        } else {
            TemplateSlotTarget::BeforeStatic(path)
        }
    }

    pub(crate) const fn static_parent(self) -> TemplatePath {
        match self.target() {
            TemplateSlotTarget::BeforeStatic(path) => path.parent(),
            TemplateSlotTarget::AppendChildren(path) => path,
        }
    }

    pub(crate) const fn fill_depth(self) -> usize {
        match self.target() {
            TemplateSlotTarget::BeforeStatic(path) => path.len(),
            TemplateSlotTarget::AppendChildren(path) => path.len() + 1,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_paths_are_single_marker_bits() {
        assert_eq!(TemplatePath::root(0).bits(), 0b1);
        assert_eq!(TemplatePath::root(1).bits(), 0b10);
        assert_eq!(TemplatePath::root(3).bits(), 0b1000);
        assert_eq!(TemplatePath::root(127).bits(), 1u128 << 127);
        assert_eq!(TemplatePath::root(128).bits(), 0);
    }

    #[test]
    fn len_counts_path_segments() {
        assert_eq!(TemplatePath::empty().len(), 0);
        assert_eq!(TemplatePath::from_bits(0b1).len(), 1);
        assert_eq!(TemplatePath::from_bits(0b100101).len(), 3);
    }

    #[test]
    fn segment_reads_sibling_indexes() {
        let path = TemplatePath::root(2)
            .next_child()
            .next_sibling()
            .next_child();

        assert_eq!(path.bits(), 0b100101);
        assert_eq!(path.segment(0), 2);
        assert_eq!(path.segment(1), 1);
        assert_eq!(path.segment(2), 0);
    }

    #[test]
    #[should_panic(expected = "bad path segment")]
    fn segment_panics_for_missing_index() {
        let _ = TemplatePath::root(0).segment(1);
    }

    #[test]
    fn starts_with_matches_segment_prefixes() {
        let root_zero = TemplatePath::root(0);
        let root_one = TemplatePath::root(1);
        let root_zero_child_one = root_zero.next_child().next_sibling();
        let root_one_child_zero = root_one.next_child();

        assert!(root_zero.starts_with(TemplatePath::empty()));
        assert!(root_zero.starts_with(root_zero));
        assert!(root_zero_child_one.starts_with(root_zero));
        assert!(root_one_child_zero.starts_with(root_one));

        assert!(!root_one.starts_with(root_zero));
        assert!(!root_one_child_zero.starts_with(root_zero));
        assert!(!root_zero.starts_with(root_zero_child_one));
    }

    #[test]
    fn starts_with_rejects_raw_bit_prefixes_across_segment_boundaries() {
        assert!(!TemplatePath::from_bits(0b10).starts_with(TemplatePath::from_bits(0b1)));
        assert!(!TemplatePath::from_bits(0b101).starts_with(TemplatePath::from_bits(0b1)));
        assert!(!TemplatePath::from_bits(0b1001).starts_with(TemplatePath::from_bits(0b10)));

        assert!(TemplatePath::from_bits(0b110).starts_with(TemplatePath::from_bits(0b1)));
        assert!(TemplatePath::from_bits(0b101).starts_with(TemplatePath::from_bits(0b10)));
    }

    #[test]
    fn split_insertion_returns_parent_and_sibling_index() {
        let path = TemplatePath::root(1)
            .next_child()
            .next_sibling()
            .next_sibling();
        let (parent, index) = path.split_insertion();

        assert_eq!(path.bits(), 0b10100);
        assert_eq!(parent.bits(), TemplatePath::root(1).bits());
        assert_eq!(index, 2);
    }
}
