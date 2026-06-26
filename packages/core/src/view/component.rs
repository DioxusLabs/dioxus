//! [`View`] implementations for [`VComponent`].

use crate::{DynamicNode, DynamicValues, VComponent, VNode, nodes::IntoVNode};
use dioxus_core_template::TemplateRawTree;

use super::{View, ViewExt, ViewTemplate};

impl ViewTemplate for VComponent {
    const TEMPLATE_TREE: &'static TemplateRawTree = &TemplateRawTree::DynamicNode;
}

impl View for VComponent {
    fn push(self, dynamic: &mut DynamicValues) {
        dynamic.push_node(DynamicNode::Component(self));
    }
}

impl IntoVNode for VComponent {
    #[inline]
    fn into_vnode(self) -> VNode {
        ViewExt::into_vnode(self)
    }
}
