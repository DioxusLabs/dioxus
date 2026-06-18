use std::num::NonZeroU128;

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
    pub fn segment(self, index: usize) -> u8 {
        let mut current_segment = 0usize;
        let mut current_index = 0u8;
        let mut started = false;
        let mut next_bit = self.bit_len();
        while next_bit > 0 {
            next_bit -= 1;
            let bit = (self.path >> next_bit) & 1;
            if bit == 1 {
                if started {
                    if current_segment == index {
                        return current_index;
                    }
                    current_segment += 1;
                    current_index = 0;
                } else {
                    started = true;
                }
            } else {
                current_index = current_index.checked_add(1).expect("path overflow");
            }
        }
        if started && current_segment == index {
            return current_index;
        }
        panic!("bad path segment");
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
}

/// A tagged dynamic node slot target.
///
/// The low bit is the target kind. The remaining high bits are a [`TemplatePath`] payload.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct TemplateSlotPath(NonZeroU128);

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

    /// Create a dynamic slot target before a static node.
    pub(crate) const fn before_static(path: TemplatePath) -> Self {
        if path.is_empty() {
            panic!("bad slot target");
        }
        Self::new(Self::encode_payload(path))
    }

    /// Create a dynamic slot target that appends to a parent.
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
    pub(crate) const fn fill_depth(self) -> usize {
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
