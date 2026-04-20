use dioxus_core::{
    Attribute, AttributeValue, DynamicNode, TemplateAttribute, TemplateNode, VNode, VirtualDom,
};
use dioxus_rsx::{Attribute as RsxAttribute, BodyNode, TextNode};
use dioxus_rsx_rosetta::{rsx_from_html, Dom};
use syn::parse_quote;

use super::{
    component_node, element_node, expr_attribute, format_rsx_nodes, is_internal_attribute_name,
    placeholder_node, text_attribute, walk_template_attrs, TemplateAttrRef,
    DANGEROUS_INNER_HTML_ATTRIBUTE, STYLE_NAMESPACE,
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
            let mut child_items = dangerous_inner_html_nodes(attrs, vnode);
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

    let dynamic_groups = attrs.iter().filter_map(|attr| match attr {
        TemplateAttribute::Dynamic { id } => Some(&*vnode.dynamic_attrs[*id]),
        _ => None,
    });

    let style = walk_template_attrs(attrs, dynamic_groups, |item| match item {
        TemplateAttrRef::Static {
            name,
            value,
            namespace,
        } => {
            if let Some(attr) = render_static_template_attribute(name, value, namespace) {
                rendered.push(attr);
            }
        }
        TemplateAttrRef::Dynamic(attr) => {
            if let Some(attr) = render_dynamic_template_attribute(attr) {
                rendered.push(attr);
            }
        }
    });

    if let Some(style) = style {
        rendered.push(text_attribute(STYLE_NAMESPACE, &style));
    }

    rendered
}

pub(crate) fn serialize_dangerous_inner_html(
    attrs: &'static [TemplateAttribute],
    vnode: &VNode,
) -> Option<String> {
    dangerous_inner_html_value(attrs, vnode)
        .map(parse_inner_html_nodes)
        .map(format_rsx_nodes)
}

fn render_static_template_attribute(
    name: &str,
    value: &str,
    namespace: Option<&str>,
) -> Option<RsxAttribute> {
    if is_internal_attribute_name(name)
        || name == DANGEROUS_INNER_HTML_ATTRIBUTE
        || matches!(namespace, Some(ns) if ns == STYLE_NAMESPACE)
    {
        return None;
    }

    Some(text_attribute(name, value))
}

fn render_dynamic_template_attribute(attr: &Attribute) -> Option<RsxAttribute> {
    if is_internal_attribute_name(attr.name)
        || attr.name == DANGEROUS_INNER_HTML_ATTRIBUTE
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

fn dangerous_inner_html_nodes(attrs: &'static [TemplateAttribute], vnode: &VNode) -> Vec<BodyNode> {
    dangerous_inner_html_value(attrs, vnode)
        .map(parse_inner_html_nodes)
        .unwrap_or_default()
}

fn dangerous_inner_html_value<'a>(
    attrs: &'static [TemplateAttribute],
    vnode: &'a VNode,
) -> Option<String> {
    let mut static_inner_html = None;
    let mut dynamic_inner_html = None;

    for attr in attrs {
        match attr {
            TemplateAttribute::Static {
                name,
                value,
                namespace: None,
            } if *name == DANGEROUS_INNER_HTML_ATTRIBUTE => {
                static_inner_html = Some((*value).to_string());
            }
            TemplateAttribute::Dynamic { id } => {
                for attr in vnode.dynamic_attrs[*id].iter() {
                    if attr.name == DANGEROUS_INNER_HTML_ATTRIBUTE {
                        dynamic_inner_html =
                            Some(dangerous_inner_html_value_to_string(&attr.value));
                    }
                }
            }
            _ => {}
        }
    }

    static_inner_html.or(dynamic_inner_html)
}

fn dangerous_inner_html_value_to_string(value: &AttributeValue) -> String {
    match value {
        AttributeValue::Text(value) => value.clone(),
        AttributeValue::Float(value) => value.to_string(),
        AttributeValue::Int(value) => value.to_string(),
        AttributeValue::Bool(value) => value.to_string(),
        AttributeValue::Any(_) | AttributeValue::Listener(_) | AttributeValue::None => {
            String::new()
        }
    }
}

fn parse_inner_html_nodes(inner_html: String) -> Vec<BodyNode> {
    if inner_html.is_empty() {
        return Vec::new();
    }

    Dom::parse(&inner_html)
        .ok()
        .map(|dom| rsx_from_html(&dom))
        .map(|body| body.body.roots)
        .unwrap_or_else(|| vec![BodyNode::Text(TextNode::from_text(&inner_html))])
}

