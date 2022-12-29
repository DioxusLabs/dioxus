use dioxus_rsx::{BodyNode, CallBody, Element, ElementAttr, ElementAttrNamed, IfmtInput};
pub use html_parser::{Dom, Node};
use proc_macro2::{Ident, Span};
use syn::LitStr;

pub fn convert_from_html(dom: Dom) -> CallBody {
    CallBody {
        roots: dom
            .children
            .into_iter()
            .filter_map(create_body_node_from_node)
            .collect(),
    }
}

fn create_body_node_from_node(node: Node) -> Option<BodyNode> {
    match node {
        Node::Text(text) => Some(BodyNode::Text(ifmt_from_text(text))),
        Node::Element(el) => {
            use convert_case::{Case, Casing};

            let el_name = el.name.to_case(Case::Snake);
            let el_name = Ident::new(el_name.as_str(), Span::call_site());

            let mut attributes: Vec<_> = el
                .attributes
                .into_iter()
                .map(|(name, value)| {
                    let ident = if matches!(name.as_str(), "for" | "async" | "type" | "as") {
                        Ident::new_raw(name.as_str(), Span::call_site())
                    } else {
                        let new_name = name.to_case(Case::Snake);
                        Ident::new(new_name.as_str(), Span::call_site())
                    };

                    ElementAttrNamed {
                        attr: ElementAttr::AttrText {
                            name: ident,
                            value: ifmt_from_text(value.unwrap_or_else(|| "false".to_string())),
                        },
                        el_name: el_name.clone(),
                    }
                })
                .collect();

            let class = el.classes.join(" ");
            if !class.is_empty() {
                attributes.push(ElementAttrNamed {
                    attr: ElementAttr::AttrText {
                        name: Ident::new("class", Span::call_site()),
                        value: ifmt_from_text(class),
                    },
                    el_name: el_name.clone(),
                });
            }

            if let Some(id) = el.id {
                attributes.push(ElementAttrNamed {
                    attr: ElementAttr::AttrText {
                        name: Ident::new("id", Span::call_site()),
                        value: ifmt_from_text(id),
                    },
                    el_name: el_name.clone(),
                });
            }

            let children = el
                .children
                .into_iter()
                .filter_map(create_body_node_from_node)
                .collect();

            Some(BodyNode::Element(Element {
                name: el_name,
                children,
                attributes,
                _is_static: false,
                key: None,
            }))
        }
        Node::Comment(_) => None,
    }
}

fn ifmt_from_text(text: String) -> IfmtInput {
    IfmtInput {
        source: Some(LitStr::new(text.as_str(), Span::call_site())),
        segments: vec![],
    }
}
