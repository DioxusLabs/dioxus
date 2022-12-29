use convert_case::{Case, Casing};
use dioxus_rsx::{BodyNode, CallBody, Element, ElementAttr, ElementAttrNamed, IfmtInput};
pub use html_parser::{Dom, Node};
use proc_macro2::{Ident, Span};
use syn::LitStr;

/// Convert an HTML DOM tree into an RSX CallBody
pub fn rsx_from_html(dom: &Dom) -> CallBody {
    CallBody {
        roots: dom.children.iter().filter_map(rsx_node_from_html).collect(),
    }
}

/// Convert an HTML Node into an RSX BodyNode
///
/// If the node is a comment, it will be ignored since RSX doesn't support comments
pub fn rsx_node_from_html(node: &Node) -> Option<BodyNode> {
    match node {
        Node::Text(text) => Some(BodyNode::Text(ifmt_from_text(text))),
        Node::Element(el) => {
            let el_name = el.name.to_case(Case::Snake);
            let el_name = Ident::new(el_name.as_str(), Span::call_site());

            let mut attributes: Vec<_> = el
                .attributes
                .iter()
                .map(|(name, value)| {
                    let ident = if matches!(name.as_str(), "for" | "async" | "type" | "as") {
                        Ident::new_raw(name.as_str(), Span::call_site())
                    } else {
                        let new_name = name.to_case(Case::Snake);
                        Ident::new(new_name.as_str(), Span::call_site())
                    };

                    ElementAttrNamed {
                        el_name: el_name.clone(),
                        attr: ElementAttr::AttrText {
                            value: ifmt_from_text(value.as_deref().unwrap_or("false")),
                            name: ident,
                        },
                    }
                })
                .collect();

            let class = el.classes.join(" ");
            if !class.is_empty() {
                attributes.push(ElementAttrNamed {
                    el_name: el_name.clone(),
                    attr: ElementAttr::AttrText {
                        name: Ident::new("class", Span::call_site()),
                        value: ifmt_from_text(&class),
                    },
                });
            }

            if let Some(id) = &el.id {
                attributes.push(ElementAttrNamed {
                    el_name: el_name.clone(),
                    attr: ElementAttr::AttrText {
                        name: Ident::new("id", Span::call_site()),
                        value: ifmt_from_text(id),
                    },
                });
            }

            let children = el.children.iter().filter_map(rsx_node_from_html).collect();

            Some(BodyNode::Element(Element {
                name: el_name,
                children,
                attributes,
                _is_static: false,
                key: None,
            }))
        }

        // We ignore comments
        Node::Comment(_) => None,
    }
}

fn ifmt_from_text(text: &str) -> IfmtInput {
    IfmtInput {
        source: Some(LitStr::new(text, Span::call_site())),
        segments: vec![],
    }
}
