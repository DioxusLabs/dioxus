use std::fmt::Arguments;

use super::ElementBuilder;
use crate::into_attr::*;
use bumpalo::collections::Vec as BumpVec;
use dioxus_core::{
    self, exports::bumpalo, Attribute, IntoVNode, Listener, NodeFactory, ScopeState, VNode,
};

pub fn fragment<'a, 'b, 'c>(
    cx: &'a ScopeState,
    node_iter: impl IntoIterator<Item = impl IntoVNode<'a> + 'c> + 'b,
) -> VNode<'a> {
    let fac = NodeFactory::new(cx);
    fac.fragment_from_iter(node_iter)
}
