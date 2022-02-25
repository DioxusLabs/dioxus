use dioxus_core::{IntoVNode, NodeFactory, ScopeState, VNode};

pub fn fragment<'a, 'b, 'c>(
    cx: &'a ScopeState,
    node_iter: impl IntoIterator<Item = impl IntoVNode<'a> + 'c> + 'b,
) -> VNode<'a> {
    let fac = NodeFactory::new(cx);
    fac.fragment_from_iter(node_iter)
}
