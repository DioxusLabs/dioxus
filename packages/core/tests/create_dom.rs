//! Regression tests for direct VNode construction paths not covered by the fuzz model.

use dioxus::prelude::*;
use dioxus_renderer_oracle::RendererOracle;

#[test]
fn empty_fragment_root_via_direct_vnode_api_is_diffable() {
    // `VNode::new` accepts `DynamicValue::Node(DynamicNode::Fragment(Vec::new()))` without
    // `DynamicNode::Placeholder(..)` so the diff path never sees an empty fragment.
    // Without that normalization, callers using the direct `VNode::new(..)` API would
    // bypass the rsx macro's `IntoDynNode` collapse and trip
    // `index out of bounds: the len is 0 but the index is 0` on the second rerender.
    use dioxus_core::{DynamicNode, DynamicValue, ScopeId, VNode, VirtualDom};

    fn app() -> Element {
        let template = VNode::placeholder().template;
        Ok(VNode::new(
            None,
            template,
            Box::new([DynamicValue::Node(DynamicNode::Fragment(Vec::new()))]),
        ))
    }

    let mut vdom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut vdom);
    vdom.mark_dirty(ScopeId::APP);
    oracle.render(&mut vdom);
    vdom.mark_dirty(ScopeId::APP);
    oracle.render(&mut vdom);
}
