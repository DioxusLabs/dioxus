use std::any::TypeId;

use anymap::AnyMap;
use dioxus_core::{Attribute, VElement};

#[derive(Debug)]
pub struct NodeView<'a> {
    inner: Option<&'a VElement<'a>>,
    view: NodeMask,
}
impl<'a> NodeView<'a> {
    pub fn new(velement: Option<&'a VElement<'a>>, view: NodeMask) -> Self {
        Self {
            inner: velement,
            view: view,
        }
    }

    pub fn tag(&self) -> Option<&'a str> {
        if self.view.tag {
            self.inner.map(|el| el.tag)
        } else {
            None
        }
    }

    pub fn namespace(&self) -> Option<&'a str> {
        if self.view.namespace {
            self.inner.map(|el| el.namespace).flatten()
        } else {
            None
        }
    }

    pub fn attributes(&self) -> impl Iterator<Item = &Attribute<'a>> {
        self.inner
            .map(|el| el.attributes)
            .unwrap_or_default()
            .iter()
            .filter(|a| self.view.attritutes.contains_attribute(&a.name))
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum AttributeMask {
    All,
    Dynamic(Vec<&'static str>),
    Static(&'static [&'static str]),
}

impl AttributeMask {
    pub const NONE: Self = Self::Static(&[]);

    fn contains_attribute(&self, attr: &'static str) -> bool {
        match self {
            AttributeMask::All => true,
            AttributeMask::Dynamic(l) => l.contains(&attr),
            AttributeMask::Static(l) => l.contains(&attr),
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
        pub fn union_iter(
            s_iter: impl Iterator<Item = &'static str>,
            o_iter: impl Iterator<Item = &'static str>,
        ) -> Vec<&'static str> {
            let mut s_peekable = s_iter.peekable();
            let mut o_peekable = o_iter.peekable();
            let mut v = Vec::new();
            while let Some(s_i) = s_peekable.peek() {
                loop {
                    if let Some(o_i) = o_peekable.peek() {
                        if o_i > s_i {
                            break;
                        } else {
                            v.push(o_peekable.next().unwrap());
                        }
                    } else {
                        break;
                    }
                }
                v.push(s_peekable.next().unwrap());
            }
            while let Some(o_i) = o_peekable.next() {
                v.push(o_i);
            }
            v
        }

        let new = match (self, other) {
            (AttributeMask::Dynamic(s), AttributeMask::Dynamic(o)) => {
                AttributeMask::Dynamic(union_iter(s.iter().copied(), o.iter().copied()))
            }
            (AttributeMask::Static(s), AttributeMask::Dynamic(o)) => {
                AttributeMask::Dynamic(union_iter(s.iter().copied(), o.iter().copied()))
            }
            (AttributeMask::Dynamic(s), AttributeMask::Static(o)) => {
                AttributeMask::Dynamic(union_iter(s.iter().copied(), o.iter().copied()))
            }
            (AttributeMask::Static(s), AttributeMask::Static(o)) => {
                AttributeMask::Dynamic(union_iter(s.iter().copied(), o.iter().copied()))
            }
            _ => AttributeMask::All,
        };
        new.verify();
        new
    }

    fn overlaps(&self, other: &Self) -> bool {
        fn overlaps_iter(
            mut self_iter: impl Iterator<Item = &'static str>,
            mut other_iter: impl Iterator<Item = &'static str>,
        ) -> bool {
            if let Some(mut other_attr) = other_iter.next() {
                while let Some(self_attr) = self_iter.next() {
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

#[derive(Default, PartialEq, Clone, Debug)]
pub struct NodeMask {
    // must be sorted
    attritutes: AttributeMask,
    tag: bool,
    namespace: bool,
}

impl NodeMask {
    pub const NONE: Self = Self::new(AttributeMask::Static(&[]), false, false);
    pub const ALL: Self = Self::new(AttributeMask::All, true, true);

    /// attritutes must be sorted!
    pub const fn new(attritutes: AttributeMask, tag: bool, namespace: bool) -> Self {
        Self {
            attritutes,
            tag,
            namespace,
        }
    }

    pub fn overlaps(&self, other: &Self) -> bool {
        (self.tag && other.tag)
            || (self.namespace && other.namespace)
            || self.attritutes_overlap(other)
    }

    fn attritutes_overlap(&self, other: &Self) -> bool {
        self.attritutes.overlaps(&other.attritutes)
    }
}

/// This state is derived from children. For example a node's size could be derived from the size of children.
/// Called when the current node's node properties are modified, a child's [BubbledUpState] is modified or a child is removed.
/// Called at most once per update.
pub trait ChildDepState {
    /// The context is passed to the [PushedDownState::reduce] when it is pushed down.
    /// This is sometimes nessisary for lifetime purposes.
    type Ctx;
    type DepState: ChildDepState;
    const NODE_MASK: NodeMask = NodeMask::new(AttributeMask::NONE, false, false);
    fn reduce(&mut self, node: NodeView, children: Vec<&Self::DepState>, ctx: &Self::Ctx) -> bool;
}

/// This state that is passed down to children. For example text properties (`<b>` `<i>` `<u>`) would be passed to children.
/// Called when the current node's node properties are modified or a parrent's [PushedDownState] is modified.
/// Called at most once per update.
pub trait ParentDepState {
    /// The context is passed to the [PushedDownState::reduce] when it is pushed down.
    /// This is sometimes nessisary for lifetime purposes.
    type Ctx;
    type DepState: ParentDepState;
    const NODE_MASK: NodeMask = NodeMask::new(AttributeMask::NONE, false, false);
    fn reduce(&mut self, node: NodeView, parent: Option<&Self::DepState>, ctx: &Self::Ctx) -> bool;
}

/// This state that is upadated lazily. For example any propertys that do not effect other parts of the dom like bg-color.
/// Called when the current node's node properties are modified or a parrent's [PushedDownState] is modified.
/// Called at most once per update.
pub trait NodeDepState {
    type Ctx;
    const NODE_MASK: NodeMask = NodeMask::new(AttributeMask::NONE, false, false);
    fn reduce(&mut self, node: NodeView, ctx: &Self::Ctx) -> bool;
}

pub trait State: Default + Clone {
    fn update_node_dep_state<'a>(
        &'a mut self,
        ty: TypeId,
        node: Option<&'a VElement<'a>>,
        ctx: &AnyMap,
    ) -> bool;
    /// This must be a valid resolution order. (no nodes updated before a state they rely on)
    fn child_dep_types(&self, mask: &NodeMask) -> Vec<TypeId>;

    fn update_parent_dep_state<'a>(
        &'a mut self,
        ty: TypeId,
        node: Option<&'a VElement<'a>>,
        parent: Option<&Self>,
        ctx: &AnyMap,
    ) -> bool;
    /// This must be a valid resolution order. (no nodes updated before a state they rely on)
    fn parent_dep_types(&self, mask: &NodeMask) -> Vec<TypeId>;

    fn update_child_dep_state<'a>(
        &'a mut self,
        ty: TypeId,
        node: Option<&'a VElement<'a>>,
        children: &[&Self],
        ctx: &AnyMap,
    ) -> bool;
    /// This must be a valid resolution order. (no nodes updated before a state they rely on)
    fn node_dep_types(&self, mask: &NodeMask) -> Vec<TypeId>;
}
