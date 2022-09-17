use dioxus_core::*;

use crate::{
    real_dom::{NodeData, NodeType, OwnedAttributeView},
    state::union_ordered_iter,
};

#[derive(Debug)]
pub struct NodeView<'a> {
    inner: &'a NodeData,
    mask: NodeMask,
}

impl<'a> NodeView<'a> {
    pub fn new(node: &'a NodeData, view: NodeMask) -> Self {
        Self {
            inner: node,
            mask: view,
        }
    }

    pub fn id(&self) -> GlobalNodeId {
        self.inner.id
    }

    pub fn tag(&self) -> Option<&'a str> {
        self.mask
            .tag
            .then(|| match &self.inner.node_type {
                NodeType::Element { tag, .. } => Some(&**tag),
                _ => None,
            })
            .flatten()
    }

    pub fn namespace(&self) -> Option<&'a str> {
        self.mask
            .namespace
            .then(|| match &self.inner.node_type {
                NodeType::Element { namespace, .. } => namespace.map(|s| &*s),
                _ => None,
            })
            .flatten()
    }

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

    pub fn text(&self) -> Option<&str> {
        self.mask
            .text
            .then(|| match &self.inner.node_type {
                NodeType::Text { text } => Some(&**text),
                _ => None,
            })
            .flatten()
    }

    pub fn listeners(&self) -> &'a [&'static str] {
        if self.mask.listeners {
            match &self.inner.node_type {
                NodeType::Element { listeners, .. } => listeners,
                _ => &[],
            }
        } else {
            &[]
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum AttributeMask {
    All,
    Dynamic(Vec<&'static str>),
    Static(&'static [&'static str]),
}

impl AttributeMask {
    pub const NONE: Self = Self::Static(&[]);

    fn contains_attribute(&self, attr: &str) -> bool {
        match self {
            AttributeMask::All => true,
            AttributeMask::Dynamic(l) => l.binary_search(&attr).is_ok(),
            AttributeMask::Static(l) => l.binary_search(&attr).is_ok(),
        }
    }

    pub fn single(new: &'static str) -> Self {
        Self::Dynamic(vec![new])
    }

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

#[derive(Default, PartialEq, Eq, Clone, Debug)]
pub struct NodeMask {
    // must be sorted
    attritutes: AttributeMask,
    tag: bool,
    namespace: bool,
    text: bool,
    listeners: bool,
}

impl NodeMask {
    pub const NONE: Self = Self::new();
    pub const ALL: Self = Self::new_with_attrs(AttributeMask::All)
        .with_text()
        .with_element();

    pub fn overlaps(&self, other: &Self) -> bool {
        (self.tag && other.tag)
            || (self.namespace && other.namespace)
            || self.attritutes.overlaps(&other.attritutes)
            || (self.text && other.text)
            || (self.listeners && other.listeners)
    }

    pub fn union(&self, other: &Self) -> Self {
        Self {
            attritutes: self.attritutes.union(&other.attritutes),
            tag: self.tag | other.tag,
            namespace: self.namespace | other.namespace,
            text: self.text | other.text,
            listeners: self.listeners | other.listeners,
        }
    }

    pub const fn new_with_attrs(attritutes: AttributeMask) -> Self {
        Self {
            attritutes,
            tag: false,
            namespace: false,
            text: false,
            listeners: false,
        }
    }

    pub const fn new() -> Self {
        Self::new_with_attrs(AttributeMask::NONE)
    }

    pub const fn with_tag(mut self) -> Self {
        self.tag = true;
        self
    }

    pub const fn with_namespace(mut self) -> Self {
        self.namespace = true;
        self
    }

    pub const fn with_element(self) -> Self {
        self.with_namespace().with_tag()
    }

    pub const fn with_text(mut self) -> Self {
        self.text = true;
        self
    }

    pub const fn with_listeners(mut self) -> Self {
        self.listeners = true;
        self
    }
}
