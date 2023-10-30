#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

use convert_case::{Case, Casing};
use dioxus_rsx::{
    BodyNode, CallBody, Component, Element, ElementAttr, ElementAttrNamed, ElementName, IfmtInput,
};
pub use html_parser::{Dom, Node};
use proc_macro2::{Ident, Span};
use syn::{punctuated::Punctuated, LitStr};

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
            let el_name = ElementName::Ident(Ident::new(el_name.as_str(), Span::call_site()));

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
                key: None,
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
            BodyNode::Component(comp) => collect_svgs(&mut comp.children, out),

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
                    prop_gen_args: None,
                    fields: vec![],
                    children: vec![],
                    manual_props: None,
                    brace: Default::default(),
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
        source: Some(LitStr::new(text, Span::call_site())),
        segments: vec![],
    }
}
