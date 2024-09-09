use crate::innerlude::*;

pub trait Visit {
    fn visit_node(&mut self, vdom: &VirtualDom, vnode: &VNode, node: TemplateNode) {
        match node {
            TemplateNode::Element {
                tag,
                namespace,
                attrs,
                children,
            } => {
                self.visit_element(vdom, vnode, tag, namespace, attrs, children);
            }
            TemplateNode::Text { text } => {
                self.visit_text(vdom, vnode, text);
            }
            TemplateNode::Dynamic { id } => {
                let node = vnode.dynamic_nodes.get(id).unwrap();
                self.visit_dynamic(vdom, vnode, id, node);
            }
        }
    }

    fn visit_element(
        &mut self,
        vdom: &VirtualDom,
        vnode: &VNode,
        tag: &'static str,
        namespace: Option<&'static str>,
        attrs: &'static [TemplateAttribute],
        children: &'static [TemplateNode],
    ) {
        for attr in attrs.iter() {
            self.visit_attr(vdom, vnode, attr);
        }

        for child in children.iter() {
            self.visit_node(vdom, vnode, *child);
        }
    }

    fn visit_attr(&mut self, vdom: &VirtualDom, vnode: &VNode, attr: &TemplateAttribute) {}

    fn visit_dynamic(
        &mut self,
        vdom: &VirtualDom,
        vnode: &VNode,
        index: usize,
        node: &DynamicNode,
    ) {
        match node {
            DynamicNode::Component(component) => {
                let scope = component.mounted_scope(index, vnode, vdom).unwrap();
                let root_node = scope.root_node();

                for root in root_node.template.roots {
                    self.visit_node(vdom, vnode, *root)
                }
            }
            DynamicNode::Text(text) => self.visit_text(vdom, vnode, &text.value),
            DynamicNode::Placeholder(_) => self.visit_placeholder(vdom, vnode, index),
            DynamicNode::Fragment(roots) => self.visit_fragment(vdom, vnode, roots),
        }
    }

    fn visit_placeholder(&mut self, vdom: &VirtualDom, vnode: &VNode, index: usize) {
        let node = vnode.dynamic_nodes.get(index).unwrap();
        self.visit_dynamic(vdom, vnode, index, node);
    }

    fn visit_fragment(&mut self, vdom: &VirtualDom, vnode: &VNode, roots: &[VNode]) {
        for root_vnode in roots {
            for root_node in root_vnode.template.roots {
                self.visit_node(vdom, vnode, *root_node)
            }
        }
    }

    fn visit_text(&mut self, vdom: &VirtualDom, vnode: &VNode, text: &str) {}
}
