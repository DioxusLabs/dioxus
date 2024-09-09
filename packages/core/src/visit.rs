use crate::innerlude::*;

/// Visitor for a [`VNode`].
pub trait Visit {
    /// Visit a [`VNode`].
    fn visit_vnode(&mut self, vdom: &VirtualDom, vnode: &VNode) {
        visit_vnode(self, vdom, vnode)
    }

    /// Visit a template node.
    fn visit_node(&mut self, vdom: &VirtualDom, vnode: &VNode, node: TemplateNode) {
        visit_node(self, vdom, vnode, node)
    }

    /// Visit an element.
    fn visit_element(
        &mut self,
        vdom: &VirtualDom,
        vnode: &VNode,
        tag: &'static str,
        namespace: Option<&'static str>,
        attrs: &'static [TemplateAttribute],
        children: &'static [TemplateNode],
    ) {
        visit_element(self, vdom, vnode, tag, namespace, attrs, children)
    }

    /// Visit an element attribute.
    fn visit_attr(&mut self, vdom: &VirtualDom, vnode: &VNode, attr: &TemplateAttribute) {
        let _ = vdom;
        let _ = vnode;
        let _ = attr;
    }

    /// Visit a dynamic node.
    fn visit_dynamic(
        &mut self,
        vdom: &VirtualDom,
        vnode: &VNode,
        index: usize,
        node: &DynamicNode,
    ) {
        visit_dynamic(self, vdom, vnode, index, node)
    }

    /// Visit a placeholder.
    fn visit_placeholder(&mut self, vdom: &VirtualDom, vnode: &VNode, index: usize) {
        visit_placeholder(self, vdom, vnode, index)
    }

    /// Visit a fragment.
    fn visit_fragment(&mut self, vdom: &VirtualDom, vnode: &VNode, roots: &[VNode]) {
        visit_fragment(self, vdom, vnode, roots)
    }

    /// Visit a text node.
    fn visit_text(&mut self, vdom: &VirtualDom, vnode: &VNode, text: &str) {
        let _ = vdom;
        let _ = vnode;
        let _ = text;
    }
}

/// Default method to visit a [`VNode`].
pub fn visit_vnode<V: Visit + ?Sized>(visitor: &mut V, vdom: &VirtualDom, vnode: &VNode) {
    for root in vnode.template.roots {
        visitor.visit_node(vdom, vnode, *root);
    }
}

/// Default method to visit a template node.
pub fn visit_node<V: Visit + ?Sized>(
    visitor: &mut V,
    vdom: &VirtualDom,
    vnode: &VNode,
    node: TemplateNode,
) {
    match node {
        TemplateNode::Element {
            tag,
            namespace,
            attrs,
            children,
        } => {
            visitor.visit_element(vdom, vnode, tag, namespace, attrs, children);
        }
        TemplateNode::Text { text } => {
            visitor.visit_text(vdom, vnode, text);
        }
        TemplateNode::Dynamic { id } => {
            let node = vnode.dynamic_nodes.get(id).unwrap();
            visitor.visit_dynamic(vdom, vnode, id, node);
        }
    }
}

/// Default method to visit an element.
pub fn visit_element<V: Visit + ?Sized>(
    visitor: &mut V,
    vdom: &VirtualDom,
    vnode: &VNode,
    tag: &'static str,
    namespace: Option<&'static str>,
    attrs: &'static [TemplateAttribute],
    children: &'static [TemplateNode],
) {
    let _ = tag;
    let _ = namespace;

    for attr in attrs.iter() {
        visitor.visit_attr(vdom, vnode, attr);
    }

    for child in children.iter() {
        visitor.visit_node(vdom, vnode, *child);
    }
}

/// Default method to visit a dynamic node.
pub fn visit_dynamic<V: Visit + ?Sized>(
    visitor: &mut V,
    vdom: &VirtualDom,
    vnode: &VNode,
    index: usize,
    node: &DynamicNode,
) {
    match node {
        DynamicNode::Component(component) => {
            let scope = component.mounted_scope(index, vnode, vdom).unwrap();
            let root_node = scope.root_node();
            visitor.visit_vnode(vdom, root_node)
        }
        DynamicNode::Text(text) => visitor.visit_text(vdom, vnode, &text.value),
        DynamicNode::Placeholder(_) => visitor.visit_placeholder(vdom, vnode, index),
        DynamicNode::Fragment(roots) => visitor.visit_fragment(vdom, vnode, roots),
    }
}

/// Default method to visit a placeholder.
pub fn visit_placeholder<V: Visit + ?Sized>(
    visitor: &mut V,
    vdom: &VirtualDom,
    vnode: &VNode,
    index: usize,
) {
    let node = vnode.dynamic_nodes.get(index).unwrap();
    visitor.visit_dynamic(vdom, vnode, index, node);
}

/// Default method to visit a fragment.
pub fn visit_fragment<V: Visit + ?Sized>(
    visitor: &mut V,
    vdom: &VirtualDom,
    vnode: &VNode,
    roots: &[VNode],
) {
    for root_vnode in roots {
        for root_node in root_vnode.template.roots {
            visitor.visit_node(vdom, vnode, *root_node)
        }
    }
}
