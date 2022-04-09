use std::any::TypeId;

use anymap::AnyMap;
use dioxus_core::{Attribute, VElement};

#[repr(transparent)]
pub struct NodeRef<'a>(&'a VElement<'a>);
impl<'a> NodeRef<'a> {
    pub fn new(velement: &'a VElement<'a>) -> Self {
        Self(velement)
    }

    pub fn tag(&self) -> &'a str {
        self.0.tag
    }

    pub fn namespace(&self) -> Option<&'a str> {
        self.0.namespace
    }

    pub fn attributes(&self) -> &'a [Attribute<'a>] {
        self.0.attributes
    }
}

pub trait ChildDepState: PartialEq {
    type Ctx;
    type DepState: ChildDepState;
    fn reduce(&mut self, node: NodeRef, children: Vec<&Self::DepState>, ctx: &Self::Ctx);
}

pub trait ParentDepState: PartialEq {
    type Ctx;
    type DepState: ParentDepState;
    fn reduce(&mut self, node: NodeRef, parent: &Self::DepState, ctx: &Self::Ctx);
}

pub trait NodeDepState: PartialEq {
    type Ctx;
    fn reduce(&mut self, node: NodeRef, ctx: &Self::Ctx);
}

pub trait State {
    fn update_node_dep_state(&mut self, ty: TypeId, node: NodeRef, ctx: &AnyMap);
    fn child_dep_types(&self) -> Vec<TypeId>;

    fn update_parent_dep_state(&mut self, ty: TypeId, node: NodeRef, parent: &Self, ctx: &AnyMap);
    fn parent_dep_types(&self) -> Vec<TypeId>;

    fn update_child_dep_state(
        &mut self,
        ty: TypeId,
        node: NodeRef,
        children: Vec<&Self>,
        ctx: &AnyMap,
    );
    fn node_dep_types(&self) -> Vec<TypeId>;
}
