use std::any::TypeId;

use anymap::AnyMap;
use dioxus_core::{Attribute, VElement};

pub struct NodeView<'a> {
    inner: &'a VElement<'a>,
    view: NodeMask,
}
impl<'a> NodeView<'a> {
    pub fn new(velement: &'a VElement<'a>, view: NodeMask) -> Self {
        Self {
            inner: velement,
            view: view,
        }
    }

    pub fn tag(&self) -> Option<&'a str> {
        if self.view.tag {
            Some(self.inner.tag)
        } else {
            None
        }
    }

    pub fn namespace(&self) -> Option<&'a str> {
        if self.view.namespace {
            self.inner.namespace
        } else {
            None
        }
    }

    pub fn attributes(&self) -> impl Iterator<Item = &Attribute<'a>> {
        self.inner
            .attributes
            .iter()
            .filter(|a| self.view.attritutes.contains(&a.name))
    }
}

#[derive(Default)]
pub struct NodeMask {
    // must be sorted
    attritutes: &'static [&'static str],
    tag: bool,
    namespace: bool,
}

impl NodeMask {
    /// attritutes must be sorted!
    pub const fn new(attritutes: &'static [&'static str], tag: bool, namespace: bool) -> Self {
        Self {
            attritutes,
            tag,
            namespace,
        }
    }

    pub fn verify(&self) {
        debug_assert!(
            self.attritutes.windows(2).all(|w| w[0] < w[1]),
            "attritutes must be increasing"
        );
    }

    pub fn overlaps(&self, other: &Self) -> bool {
        (self.tag && other.tag)
            || (self.namespace && other.namespace)
            || self.attritutes_overlap(other)
    }

    fn attritutes_overlap(&self, other: &Self) -> bool {
        let mut self_attrs = self.attritutes.iter();
        let mut other_attrs = other.attritutes.iter();
        if let Some(mut other_attr) = other_attrs.next() {
            while let Some(self_attr) = self_attrs.next() {
                while other_attr < self_attr {
                    if let Some(attr) = other_attrs.next() {
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
}

pub trait ChildDepState {
    type Ctx;
    type DepState: ChildDepState;
    const NODE_MASK: NodeMask = NodeMask::new(&[], false, false);
    fn reduce(&mut self, node: NodeView, children: Vec<&Self::DepState>, ctx: &Self::Ctx) -> bool;
}

pub trait ParentDepState {
    type Ctx;
    type DepState: ParentDepState;
    const NODE_MASK: NodeMask = NodeMask::new(&[], false, false);
    fn reduce(&mut self, node: NodeView, parent: &Self::DepState, ctx: &Self::Ctx) -> bool;
}

pub trait NodeDepState {
    type Ctx;
    const NODE_MASK: NodeMask = NodeMask::new(&[], false, false);
    fn reduce(&mut self, node: NodeView, ctx: &Self::Ctx) -> bool;
}

pub trait State: Default {
    fn update_node_dep_state<'a>(
        &'a mut self,
        ty: TypeId,
        node: &'a VElement<'a>,
        ctx: &AnyMap,
    ) -> bool;
    /// This must be a valid resolution order. (no nodes updated before a state they rely on)
    fn child_dep_types(&self, mask: &NodeMask) -> Vec<TypeId>;

    fn update_parent_dep_state<'a>(
        &'a mut self,
        ty: TypeId,
        node: &'a VElement<'a>,
        parent: &Self,
        ctx: &AnyMap,
    ) -> bool;
    /// This must be a valid resolution order. (no nodes updated before a state they rely on)
    fn parent_dep_types(&self, mask: &NodeMask) -> Vec<TypeId>;

    fn update_child_dep_state<'a>(
        &'a mut self,
        ty: TypeId,
        node: &'a VElement<'a>,
        children: Vec<&Self>,
        ctx: &AnyMap,
    ) -> bool;
    /// This must be a valid resolution order. (no nodes updated before a state they rely on)
    fn node_dep_types(&self, mask: &NodeMask) -> Vec<TypeId>;
}
