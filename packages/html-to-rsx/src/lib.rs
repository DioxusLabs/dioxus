use dioxus_rsx::{BodyNode, CallBody, IfmtInput};
use html_parser::{Dom, Node};
use proc_macro2::Span;
use syn::LitStr;

#[derive(thiserror::Error, Debug)]
pub enum ConvertError {}

pub fn convert_from_html(html: Dom) -> CallBody {
    let roots = html
        .children
        .into_iter()
        .map(|f| create_body_node_from_node(f))
        .filter_map(|f| f)
        .collect();

    CallBody { roots }
}

fn create_body_node_from_node(node: Node) -> Option<BodyNode> {
    let res = match node {
        Node::Text(text) => BodyNode::Text(IfmtInput {
            source: Some(LitStr::new(text.as_str(), Span::call_site())),
            segments: vec![],
        }),
        Node::Element(_) => todo!(),
        Node::Comment(_) => return None,
    };

    Some(res)
}
