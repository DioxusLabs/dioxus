use dioxus_core::{
    Attribute, AttributeValue, DynamicNode, TemplateAttribute, TemplateNode, VNode, VirtualDom,
};
use dioxus_rsx::{Attribute as RsxAttribute, BodyNode, TextNode};
use syn::parse_quote;

use super::{
    component_node, element_node, expr_attribute, format_rsx_nodes, is_internal_attribute_name,
    placeholder_node, text_attribute,
};

pub(crate) fn serialize_template_subtree(
    dom: &VirtualDom,
    vnode: &VNode,
    node: &TemplateNode,
) -> String {
    format_rsx_nodes(serialize_template_node_items(dom, vnode, node))
}

pub(crate) fn serialize_vnode_subtree(dom: &VirtualDom, vnode: &VNode) -> String {
    format_rsx_nodes(serialize_vnode_items(dom, vnode))
}

fn serialize_template_node_items(
    dom: &VirtualDom,
    vnode: &VNode,
    node: &TemplateNode,
) -> Vec<BodyNode> {
    match node {
        TemplateNode::Element {
            tag,
            attrs,
            children,
            ..
        } => {
            let attributes = serialize_template_attributes(attrs, vnode);
            let mut child_items = Vec::new();
            for child in *children {
                child_items.extend(serialize_template_node_items(dom, vnode, child));
            }
            vec![element_node(tag, attributes, child_items)]
        }
        TemplateNode::Text { text } => vec![BodyNode::Text(TextNode::from_text(text))],
        TemplateNode::Dynamic { id } => serialize_dynamic_node_items(dom, vnode, *id),
    }
}

fn serialize_dynamic_node_items(
    dom: &VirtualDom,
    vnode: &VNode,
    dynamic_id: usize,
) -> Vec<BodyNode> {
    match &vnode.dynamic_nodes[dynamic_id] {
        DynamicNode::Text(text) => vec![BodyNode::Text(TextNode::from_text(&text.value))],
        DynamicNode::Placeholder(_) => vec![placeholder_node()],
        DynamicNode::Fragment(fragment) => fragment
            .iter()
            .flat_map(|fragment_vnode| serialize_vnode_items(dom, fragment_vnode))
            .collect(),
        DynamicNode::Component(comp) => comp
            .mounted_scope(dynamic_id, vnode, dom)
            .map(|scope| serialize_vnode_items(dom, scope.root_node()))
            .unwrap_or_else(|| vec![component_node(comp.name)]),
    }
}

fn serialize_vnode_items(dom: &VirtualDom, vnode: &VNode) -> Vec<BodyNode> {
    vnode
        .template
        .roots
        .iter()
        .flat_map(|root| serialize_template_node_items(dom, vnode, root))
        .collect()
}

fn serialize_template_attributes(
    attrs: &'static [TemplateAttribute],
    vnode: &VNode,
) -> Vec<RsxAttribute> {
    let mut rendered = Vec::new();

    for attr in attrs {
        match attr {
            TemplateAttribute::Static { name, value, .. } => {
                if let Some(rendered_attr) = render_static_template_attribute(name, value) {
                    rendered.push(rendered_attr);
                }
            }
            TemplateAttribute::Dynamic { id } => {
                let mut dynamic_attrs: Vec<_> = vnode.dynamic_attrs[*id]
                    .iter()
                    .filter_map(render_dynamic_template_attribute)
                    .collect();
                dynamic_attrs.sort_by_key(|attr| attr.name.to_string());
                rendered.extend(dynamic_attrs);
            }
        }
    }

    rendered
}

fn render_static_template_attribute(name: &str, value: &str) -> Option<RsxAttribute> {
    if is_internal_attribute_name(name) {
        return None;
    }

    Some(text_attribute(name, value))
}

fn render_dynamic_template_attribute(attr: &Attribute) -> Option<RsxAttribute> {
    if is_internal_attribute_name(attr.name)
        || matches!(
            attr.value,
            AttributeValue::Listener(_) | AttributeValue::None
        )
    {
        return None;
    }

    let rendered_value = match &attr.value {
        AttributeValue::Text(value) => text_attribute(attr.name, value),
        AttributeValue::Float(value) if value.is_finite() => {
            let value = *value;
            expr_attribute(attr.name, parse_quote!(#value))
        }
        AttributeValue::Float(_) => text_attribute(attr.name, "<non-finite-float>"),
        AttributeValue::Int(value) => {
            let value = *value;
            expr_attribute(attr.name, parse_quote!(#value))
        }
        AttributeValue::Bool(value) => {
            let value = *value;
            expr_attribute(attr.name, parse_quote!(#value))
        }
        AttributeValue::Any(_) => text_attribute(attr.name, "<any>"),
        AttributeValue::Listener(_) | AttributeValue::None => return None,
    };

    Some(rendered_value)
}
