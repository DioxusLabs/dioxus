use dioxus_core::*;

use crate::{
    real_dom::{NodeData, NodeType, OwnedAttributeView},
    state::union_ordered_iter,
};

/// A view into a [VNode] with limited access.
#[derive(Debug)]
pub struct NodeView<'a> {
    inner: &'a NodeData,
    mask: NodeMask,
}

impl<'a> NodeView<'a> {
    /// Create a new NodeView from a VNode, and mask.
    pub fn new(node: &'a NodeData, view: NodeMask) -> Self {
        Self {
            inner: node,
            mask: view,
        }
    }

    /// Get the id of the node
    pub fn id(&self) -> GlobalNodeId {
        self.inner.id
    }

    /// Get the tag of the node if the tag is enabled in the mask
    pub fn tag(&self) -> Option<&'a str> {
        self.mask
            .tag
            .then(|| match &self.inner.node_type {
                NodeType::Element { tag, .. } => Some(&**tag),
                _ => None,
            })
            .flatten()
    }

    /// Get the tag of the node if the namespace is enabled in the mask
    pub fn namespace(&self) -> Option<&'a str> {
        self.mask
            .namespace
            .then(|| match &self.inner.node_type {
                NodeType::Element { namespace, .. } => namespace.map(|s| &*s),
                _ => None,
            })
            .flatten()
    }

    /// Get any attributes that are enabled in the mask
    pub fn attributes<'b>(&'b self) -> Option<impl Iterator<Item = OwnedAttributeView<'a>> + 'b> {
        match &self.inner.node_type {
            NodeType::Element { attributes, .. } => Some(
                attributes
                    .iter()
                    .filter(move |(attr, _)| self.mask.attritutes.contains_attribute(&attr.name))
                    .map(|(attr, val)| OwnedAttributeView {
                        attribute: attr,
                        value: val,
                    }),
            ),
            _ => None,
        }
    }

    /// Get the text if it is enabled in the mask
    pub fn text(&self) -> Option<&str> {
        self.mask
            .text
            .then(|| match &self.inner.node_type {
                NodeType::Text { text } => Some(&**text),
                _ => None,
            })
            .flatten()
    }

    /// Get the listeners if it is enabled in the mask
    pub fn listeners(&self) -> Option<impl Iterator<Item = &'a str> + '_> {
        if self.mask.listeners {
            match &self.inner.node_type {
                NodeType::Element { listeners, .. } => Some(listeners.iter().map(|l| &**l)),
                _ => None,
            }
        } else {
            None
        }
    }
}

/// A mask that contains a list of attributes that are visible.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum AttributeMask {
    All,
    /// A list of attribute names that are visible, this list must be sorted
    Dynamic(Vec<&'static str>),
    /// A list of attribute names that are visible, this list must be sorted
    Static(&'static [&'static str]),
}

impl AttributeMask {
    /// A empty attribute mask
    pub const NONE: Self = Self::Static(&[]);

    fn contains_attribute(&self, attr: &str) -> bool {
        match self {
            AttributeMask::All => true,
            AttributeMask::Dynamic(l) => l.binary_search(&attr).is_ok(),
            AttributeMask::Static(l) => l.binary_search(&attr).is_ok(),
        }
    }

    /// Create a new dynamic attribute mask with a single attribute
    pub fn single(new: &'static str) -> Self {
        Self::Dynamic(vec![new])
    }

    /// Ensure the attribute list is sorted.
    pub fn verify(&self) {
        match &self {
            AttributeMask::Static(attrs) => debug_assert!(
                attrs.windows(2).all(|w| w[0] < w[1]),
                "attritutes must be increasing"
            ),
            AttributeMask::Dynamic(attrs) => debug_assert!(
                attrs.windows(2).all(|w| w[0] < w[1]),
                "attritutes must be increasing"
            ),
            _ => (),
        }
    }

    /// Combine two attribute masks
    pub fn union(&self, other: &Self) -> Self {
        let new = match (self, other) {
            (AttributeMask::Dynamic(s), AttributeMask::Dynamic(o)) => AttributeMask::Dynamic(
                union_ordered_iter(s.iter().copied(), o.iter().copied(), s.len() + o.len()),
            ),
            (AttributeMask::Static(s), AttributeMask::Dynamic(o)) => AttributeMask::Dynamic(
                union_ordered_iter(s.iter().copied(), o.iter().copied(), s.len() + o.len()),
            ),
            (AttributeMask::Dynamic(s), AttributeMask::Static(o)) => AttributeMask::Dynamic(
                union_ordered_iter(s.iter().copied(), o.iter().copied(), s.len() + o.len()),
            ),
            (AttributeMask::Static(s), AttributeMask::Static(o)) => AttributeMask::Dynamic(
                union_ordered_iter(s.iter().copied(), o.iter().copied(), s.len() + o.len()),
            ),
            _ => AttributeMask::All,
        };
        new.verify();
        new
    }

    /// Check if two attribute masks overlap
    fn overlaps(&self, other: &Self) -> bool {
        fn overlaps_iter(
            self_iter: impl Iterator<Item = &'static str>,
            mut other_iter: impl Iterator<Item = &'static str>,
        ) -> bool {
            if let Some(mut other_attr) = other_iter.next() {
                for self_attr in self_iter {
                    while other_attr < self_attr {
                        if let Some(attr) = other_iter.next() {
                            other_attr = attr;
                        } else {
                            return false;
                        }
                    }
                    if other_attr == self_attr {
                        return true;
                    }
                }
            }
            false
        }
        match (self, other) {
            (AttributeMask::All, AttributeMask::All) => true,
            (AttributeMask::All, AttributeMask::Dynamic(v)) => !v.is_empty(),
            (AttributeMask::All, AttributeMask::Static(s)) => !s.is_empty(),
            (AttributeMask::Dynamic(v), AttributeMask::All) => !v.is_empty(),
            (AttributeMask::Static(s), AttributeMask::All) => !s.is_empty(),
            (AttributeMask::Dynamic(v1), AttributeMask::Dynamic(v2)) => {
                overlaps_iter(v1.iter().copied(), v2.iter().copied())
            }
            (AttributeMask::Dynamic(v), AttributeMask::Static(s)) => {
                overlaps_iter(v.iter().copied(), s.iter().copied())
            }
            (AttributeMask::Static(s), AttributeMask::Dynamic(v)) => {
                overlaps_iter(v.iter().copied(), s.iter().copied())
            }
            (AttributeMask::Static(s1), AttributeMask::Static(s2)) => {
                overlaps_iter(s1.iter().copied(), s2.iter().copied())
            }
        }
    }
}

impl Default for AttributeMask {
    fn default() -> Self {
        AttributeMask::Static(&[])
    }
}

/// A mask that limits what parts of a node a dependency can see.
#[derive(Default, PartialEq, Eq, Clone, Debug)]
pub struct NodeMask {
    attritutes: AttributeMask,
    tag: bool,
    namespace: bool,
    text: bool,
    listeners: bool,
}

impl NodeMask {
    /// A node mask with no parts visible.
    pub const NONE: Self = Self::new();
    /// A node mask with every part visible.
    pub const ALL: Self = Self::new_with_attrs(AttributeMask::All)
        .with_text()
        .with_element()
        .with_listeners();

    /// Check if two masks overlap
    pub fn overlaps(&self, other: &Self) -> bool {
        (self.tag && other.tag)
            || (self.namespace && other.namespace)
            || self.attritutes.overlaps(&other.attritutes)
            || (self.text && other.text)
            || (self.listeners && other.listeners)
    }

    /// Combine two node masks
    pub fn union(&self, other: &Self) -> Self {
        Self {
            attritutes: self.attritutes.union(&other.attritutes),
            tag: self.tag | other.tag,
            namespace: self.namespace | other.namespace,
            text: self.text | other.text,
            listeners: self.listeners | other.listeners,
        }
    }

    /// Create a new node mask with the given attributes
    pub const fn new_with_attrs(attritutes: AttributeMask) -> Self {
        Self {
            attritutes,
            tag: false,
            namespace: false,
            text: false,
            listeners: false,
        }
    }

    /// Create a empty node mask
    pub const fn new() -> Self {
        Self::new_with_attrs(AttributeMask::NONE)
    }

    /// Allow the mask to view the tag
    pub const fn with_tag(mut self) -> Self {
        self.tag = true;
        self
    }

    /// Allow the mask to view the namespace
    pub const fn with_namespace(mut self) -> Self {
        self.namespace = true;
        self
    }

    /// Allow the mask to view the namespace and tag
    pub const fn with_element(self) -> Self {
        self.with_namespace().with_tag()
    }

    /// Allow the mask to view the text
    pub const fn with_text(mut self) -> Self {
        self.text = true;
        self
    }

    /// Allow the mask to view the listeners
    pub const fn with_listeners(mut self) -> Self {
        self.listeners = true;
        self
    }
}
