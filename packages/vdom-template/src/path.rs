use std::num::NonZeroU128;

/// Maximum raw [`TemplatePath`] bits that can be stored in a [`TemplateSlotPath`].
///
/// Slot paths reserve one low tag bit and store the path payload in the remaining bits.
pub const TEMPLATE_SLOT_PATH_MAX_PATH_BITS: usize = u128::BITS as usize - 1;

/// A compact path from a template root to a static node or dynamic attribute.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TemplatePath {
    path: u128,
}

impl TemplatePath {
    /// Create an empty path.
    pub(crate) const fn empty() -> Self {
        Self { path: 0 }
    }

    /// Create a path from compact path bits.
    pub(crate) const fn from_bits(path: u128) -> Self {
        Self { path }
    }

    /// Return the path for a root position.
    ///
    /// Root indices are bounded well below 128 by the rsx splitter's path-bit
    /// limit, so an out-of-range index indicates a bug rather than a value to
    /// silently collapse to the empty path.
    pub(crate) const fn root(index: usize) -> Self {
        debug_assert!(
            index < u128::BITS as usize,
            "template root index exceeds path bit width"
        );

        Self {
            path: 1u128 << index,
        }
    }

    /// Return the compact path bits.
    pub(crate) const fn bits(self) -> u128 {
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
        self.depth() == 1
    }

    /// Return the number of path segments from the template root to this path.
    pub const fn depth(self) -> usize {
        self.path.count_ones() as usize
    }

    /// Return true if this path has no segments.
    pub const fn is_empty(self) -> bool {
        self.path == 0
    }

    /// Iterate sibling indexes from the root to this path.
    pub fn segments(self) -> TemplatePathSegments {
        TemplatePathSegments { path: self.path }
    }

    /// Return true if this compact path starts with `ancestor`.
    pub fn starts_with(self, ancestor: TemplatePath) -> bool {
        let ancestor_depth = ancestor.depth();
        if ancestor_depth == 0 {
            return true;
        }

        if ancestor_depth > self.depth() {
            return false;
        }

        if ancestor_depth == self.depth() {
            return self.path == ancestor.path;
        }

        let self_bits = self.bit_len();
        let ancestor_bits = ancestor.bit_len();
        if ancestor_bits > self_bits {
            return false;
        }

        let suffix_bits = self_bits - ancestor_bits;
        suffix_bits > 0
            && (self.path >> suffix_bits) == ancestor.path
            && ((self.path >> (suffix_bits - 1)) & 1) == 1
    }

    /// Return the number of raw child/sibling bits in this path.
    pub(crate) fn bit_len(self) -> u32 {
        u128::BITS - self.path.leading_zeros()
    }
}

/// Iterator over the sibling indexes encoded in a [`TemplatePath`].
#[derive(Clone, Copy)]
pub struct TemplatePathSegments {
    path: u128,
}

impl Iterator for TemplatePathSegments {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let bit_len = u128::BITS - self.path.leading_zeros();
        if bit_len == 0 {
            return None;
        }

        let marker = 1u128 << (bit_len - 1);
        let remaining_path = self.path ^ marker;
        let next_marker_bit_len = u128::BITS - remaining_path.leading_zeros();
        let segment = if next_marker_bit_len == 0 {
            bit_len - 1
        } else {
            bit_len - next_marker_bit_len - 1
        };

        self.path = remaining_path;
        Some(segment as usize)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl ExactSizeIterator for TemplatePathSegments {
    fn len(&self) -> usize {
        self.path.count_ones() as usize
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct TemplateSlotPath(NonZeroU128);

/// The static path an anchor binds to and whether that path is the last static node at its level.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TemplateSlotTarget {
    static_path: TemplatePath,
    is_last_static_node: bool,
}

impl TemplateSlotTarget {
    /// The static path this anchor binds to.
    pub const fn static_path(self) -> TemplatePath {
        self.static_path
    }

    /// Whether this anchor points at the last static node before its dynamic slot.
    pub const fn is_last_static_node(self) -> bool {
        self.is_last_static_node
    }
}

impl TemplateSlotPath {
    const TARGET_LAST_STATIC_NODE: u128 = 1;
    const MAX_PAYLOAD: u128 = u128::MAX >> (u128::BITS as usize - TEMPLATE_SLOT_PATH_MAX_PATH_BITS);

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

    pub(crate) const fn static_node(path: TemplatePath) -> Self {
        if path.is_empty() {
            panic!("bad slot target");
        }
        Self::new(Self::encode_payload(path))
    }

    pub(crate) const fn last_static_node(path: TemplatePath) -> Self {
        Self::new(Self::encode_payload(path) | Self::TARGET_LAST_STATIC_NODE)
    }

    /// Return the raw tagged bits.
    pub(crate) const fn bits(self) -> u128 {
        self.0.get()
    }

    pub(crate) const fn target(self) -> TemplateSlotTarget {
        let bits = self.bits();
        TemplateSlotTarget {
            static_path: TemplatePath::from_bits(bits >> 1),
            is_last_static_node: bits & Self::TARGET_LAST_STATIC_NODE
                == Self::TARGET_LAST_STATIC_NODE,
        }
    }

    pub(crate) const fn static_path(self) -> TemplatePath {
        self.target().static_path()
    }

    pub(crate) const fn is_last_static_node(self) -> bool {
        self.target().is_last_static_node()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_path_payload_uses_all_but_the_tag_bit() {
        assert_eq!(TEMPLATE_SLOT_PATH_MAX_PATH_BITS, 127);
        assert_eq!(TemplateSlotPath::MAX_PAYLOAD, u128::MAX >> 1);
    }

    #[test]
    fn root_paths_are_single_marker_bits() {
        assert_eq!(TemplatePath::root(0).bits(), 0b1);
        assert_eq!(TemplatePath::root(1).bits(), 0b10);
        assert_eq!(TemplatePath::root(3).bits(), 0b1000);
        assert_eq!(TemplatePath::root(127).bits(), 1u128 << 127);
    }

    #[test]
    fn depth_counts_path_segments() {
        assert_eq!(TemplatePath::empty().depth(), 0);
        assert_eq!(TemplatePath::from_bits(0b1).depth(), 1);
        assert_eq!(TemplatePath::from_bits(0b100101).depth(), 3);
    }

    #[test]
    fn segments_reads_sibling_indexes() {
        let path = TemplatePath::root(2)
            .next_child()
            .next_sibling()
            .next_child();

        assert_eq!(path.bits(), 0b100101);
        assert_eq!(path.segments().collect::<Vec<_>>(), vec![2, 1, 0]);
    }

    #[test]
    fn segments_stop_at_end() {
        let mut segments = TemplatePath::root(0).segments();
        assert_eq!(segments.next(), Some(0));
        assert_eq!(segments.next(), None);
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
        assert!(
            !TemplatePath::root(0)
                .next_child()
                .starts_with(TemplatePath::root(5))
        );

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

    #[test]
    fn slot_path_tag_marks_last_static_node() {
        let path = TemplatePath::root(0).next_child().next_sibling();
        let static_node = TemplateSlotPath::static_node(path);
        let last_static_node = TemplateSlotPath::last_static_node(path);

        assert_eq!(static_node.static_path().bits(), path.bits());
        assert!(!static_node.is_last_static_node());
        assert_eq!(last_static_node.static_path().bits(), path.bits());
        assert!(last_static_node.is_last_static_node());
    }
}
