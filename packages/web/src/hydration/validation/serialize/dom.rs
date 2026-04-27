use dioxus_rsx::{Attribute as RsxAttribute, BodyNode, TextNode};
use wasm_bindgen::JsCast;

use super::{
    element_node, format_rsx_nodes, is_internal_attribute_name, missing_node, placeholder_node,
    text_attribute,
};

pub(crate) fn serialize_dom_subtree(node: &web_sys::Node) -> String {
    format_rsx_nodes(serialize_dom_node_items(node))
}

pub(crate) fn serialize_dom_nodes(nodes: &[web_sys::Node]) -> String {
    format_rsx_nodes(
        nodes
            .iter()
            .flat_map(serialize_dom_node_items)
            .collect::<Vec<_>>(),
    )
}

fn serialize_dom_node_items(node: &web_sys::Node) -> Vec<BodyNode> {
    if should_skip_validation_node(node) {
        return Vec::new();
    }

    match node.node_type() {
        web_sys::Node::ELEMENT_NODE => {
            let Some(element) = node.dyn_ref::<web_sys::Element>() else {
                return vec![missing_node()];
            };

            let attrs = serialize_dom_attributes(element);
            let mut children = Vec::new();
            let mut child = node.first_child();
            while let Some(current) = child {
                children.extend(serialize_dom_node_items(&current));
                child = current.next_sibling();
            }

            vec![element_node(
                &element.tag_name().to_lowercase(),
                attrs,
                children,
            )]
        }
        web_sys::Node::TEXT_NODE => {
            vec![BodyNode::Text(TextNode::from_text(
                &node.text_content().unwrap_or_default(),
            ))]
        }
        web_sys::Node::COMMENT_NODE => {
            let comment = node.text_content().unwrap_or_default();
            if is_placeholder_comment(&comment) {
                vec![placeholder_node()]
            } else {
                Vec::new()
            }
        }
        _ => vec![BodyNode::Text(TextNode::from_text(&format!(
            "<node type {}>",
            node.node_type()
        )))],
    }
}

fn serialize_dom_attributes(element: &web_sys::Element) -> Vec<RsxAttribute> {
    let mut rendered = Vec::new();
    let names = element.get_attribute_names();

    for idx in 0..names.length() {
        let Some(name) = names.get(idx).as_string() else {
            continue;
        };
        if is_internal_attribute_name(&name) {
            continue;
        }
        let value = element.get_attribute(&name).unwrap_or_default();
        rendered.push(text_attribute(&name, &value));
    }

    rendered
}

pub(crate) fn should_skip_validation_node(node: &web_sys::Node) -> bool {
    if node.node_type() == web_sys::Node::COMMENT_NODE {
        let marker = node.text_content().unwrap_or_default();
        let marker = marker.trim();
        return marker.starts_with("node-id") || marker == "#" || !is_placeholder_comment(marker);
    }

    let Some(element) = node.dyn_ref::<web_sys::Element>() else {
        return false;
    };

    if !element.tag_name().eq_ignore_ascii_case("script") {
        return false;
    }

    let script = node.text_content().unwrap_or_default();
    let script = script.trim();

    script.starts_with("window.hydrate_queue=")
        || script.starts_with("window.dx_hydrate(")
        || script.starts_with("window.initial_dioxus_hydration_data=")
        || script.starts_with("window.initial_dioxus_hydration_debug_types=")
        || script.starts_with("window.initial_dioxus_hydration_debug_locations=")
}

fn is_placeholder_comment(comment: &str) -> bool {
    comment.trim().starts_with("placeholder")
}
