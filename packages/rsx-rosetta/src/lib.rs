#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

use convert_case::{Case, Casing};
use dioxus_html::{map_html_attribute_to_rsx, map_html_element_to_rsx};
use dioxus_rsx::{
    Attribute, AttributeName, AttributeValue, BodyNode, CallBody, Component, Element, ElementName,
    HotLiteral, IfmtInput, TemplateBody, TextNode,
};
pub use html_parser::{Dom, Node};
use proc_macro2::{Ident, Span};
use syn::{punctuated::Punctuated, LitStr};

/// Convert an HTML DOM tree into an RSX CallBody
pub fn rsx_from_html(dom: &Dom) -> CallBody {
    let nodes = dom
        .children
        .iter()
        .filter_map(rsx_node_from_html)
        .collect::<Vec<_>>();

    let template = TemplateBody::new(nodes);
    let body = CallBody::new(template);

    body
}

/// Convert an HTML Node into an RSX BodyNode
///
/// If the node is a comment, it will be ignored since RSX doesn't support comments
pub fn rsx_node_from_html(node: &Node) -> Option<BodyNode> {
    match node {
        Node::Text(text) => Some(BodyNode::Text(TextNode {
            input: ifmt_from_text(text),
            dyn_idx: Default::default(),
            hr_idx: Default::default(),
        })),

        Node::Element(el) => {
            let el_name = if let Some(name) = map_html_element_to_rsx(&el.name) {
                ElementName::Ident(Ident::new(name, Span::call_site()))
            } else {
                // if we don't recognize it and it has a dash, we assume it's a web component
                if el.name.contains('-') {
                    ElementName::Custom(LitStr::new(&el.name, Span::call_site()))
                } else {
                    // otherwise, it might be an element that isn't supported yet
                    ElementName::Ident(Ident::new(&el.name.to_case(Case::Snake), Span::call_site()))
                }
            };

            let mut attributes: Vec<_> = el
                .attributes
                .iter()
                .map(|(name, value)| {
                    let value = literal_from_text(value.as_deref().unwrap_or("false"));
                    let attr = if let Some(name) = map_html_attribute_to_rsx(name) {
                        let ident = if let Some(name) = name.strip_prefix("r#") {
                            Ident::new_raw(name, Span::call_site())
                        } else {
                            Ident::new(name, Span::call_site())
                        };
                        Attribute {
                            value: AttributeValue::AttrLiteral(value),
                            colon: Some(Default::default()),
                            comma: Some(Default::default()),
                            name: AttributeName::BuiltIn(ident),
                            dyn_idx: Default::default(),
                        }
                    } else {
                        // If we don't recognize the attribute, we assume it's a custom attribute
                        Attribute {
                            value: AttributeValue::AttrLiteral(value),
                            colon: Some(Default::default()),
                            name: AttributeName::Custom(LitStr::new(name, Span::call_site())),
                            comma: Some(Default::default()),
                            dyn_idx: Default::default(),
                        }
                    };

                    attr
                })
                .collect();

            let class = el.classes.join(" ");
            if !class.is_empty() {
                attributes.push(Attribute {
                    name: AttributeName::BuiltIn(Ident::new("class", Span::call_site())),
                    colon: Some(Default::default()),
                    value: AttributeValue::AttrLiteral(literal_from_text(&class)),
                    comma: Some(Default::default()),
                    dyn_idx: Default::default(),
                });
            }

            if let Some(id) = &el.id {
                attributes.push(Attribute {
                    name: AttributeName::BuiltIn(Ident::new("id", Span::call_site())),
                    colon: Some(Default::default()),
                    value: AttributeValue::AttrLiteral(literal_from_text(id)),
                    comma: Some(Default::default()),
                    dyn_idx: Default::default(),
                });
            }

            let children = el.children.iter().filter_map(rsx_node_from_html).collect();

            Some(BodyNode::Element(Element {
                name: el_name,
                children,
                raw_attributes: attributes,
                merged_attributes: Default::default(),
                diagnostics: Default::default(),
                spreads: Default::default(),
                brace: Default::default(),
            }))
        }

        // We ignore comments
        Node::Comment(_) => None,
    }
}

/// Pull out all the svgs from the body and replace them with components of the same name
pub fn collect_svgs(children: &mut [BodyNode], out: &mut Vec<BodyNode>) {
    for child in children {
        match child {
            BodyNode::Component(comp) => collect_svgs(&mut comp.children.roots, out),

            BodyNode::Element(el) if el.name == "svg" => {
                // we want to replace this instance with a component
                let mut segments = Punctuated::new();

                segments.push(Ident::new("icons", Span::call_site()).into());

                let new_name: Ident = Ident::new(&format!("icon_{}", out.len()), Span::call_site());

                segments.push(new_name.clone().into());

                // Replace this instance with a component
                let mut new_comp = BodyNode::Component(Component {
                    name: syn::Path {
                        leading_colon: None,
                        segments,
                    },
                    generics: None,
                    spreads: Default::default(),
                    diagnostics: Default::default(),
                    fields: vec![],
                    children: TemplateBody::new(vec![]),
                    brace: Default::default(),
                    dyn_idx: Default::default(),
                });

                std::mem::swap(child, &mut new_comp);

                // And push the original svg into the svg list
                out.push(new_comp);
            }

            BodyNode::Element(el) => collect_svgs(&mut el.children, out),

            _ => {}
        }
    }
}

fn ifmt_from_text(text: &str) -> IfmtInput {
    IfmtInput {
        source: LitStr::new(text, Span::call_site()),
        segments: vec![],
    }
}

fn literal_from_text(text: &str) -> HotLiteral {
    HotLiteral {
        value: dioxus_rsx::HotLiteralType::Fmted(IfmtInput {
            source: LitStr::new(text, Span::call_site()),
            segments: vec![],
        }),
        hr_idx: Default::default(),
    }
}
