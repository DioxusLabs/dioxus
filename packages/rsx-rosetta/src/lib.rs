#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

use convert_case::{Case, Casing};
use dioxus_html::{map_html_attribute_to_rsx, map_html_element_to_rsx};
use dioxus_rsx::{
    Attribute, AttributeName, AttributeValue, BodyNode, CallBody, Component, Element, ElementName,
    HotLiteral, TemplateBody, TextNode,
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

    CallBody::new(template)
}

/// Convert an HTML Node into an RSX BodyNode
///
/// If the node is a comment, it will be ignored since RSX doesn't support comments
pub fn rsx_node_from_html(node: &Node) -> Option<BodyNode> {
    use AttributeName::*;
    use AttributeValue::*;

    match node {
        Node::Text(text) => Some(BodyNode::Text(TextNode::from_text(text))),

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
                    let value = HotLiteral::from_raw_text(value.as_deref().unwrap_or("false"));
                    let attr = if let Some(name) = map_html_attribute_to_rsx(name) {
                        let name = if let Some(name) = name.strip_prefix("r#") {
                            Ident::new_raw(name, Span::call_site())
                        } else {
                            Ident::new(name, Span::call_site())
                        };
                        BuiltIn(name)
                    } else {
                        // If we don't recognize the attribute, we assume it's a custom attribute
                        Custom(LitStr::new(name, Span::call_site()))
                    };

                    Attribute::from_raw(attr, AttrLiteral(value))
                })
                .collect();

            let class = el.classes.join(" ");
            if !class.is_empty() {
                attributes.push(Attribute::from_raw(
                    BuiltIn(Ident::new("class", Span::call_site())),
                    AttrLiteral(HotLiteral::from_raw_text(&class)),
                ));
            }

            if let Some(id) = &el.id {
                attributes.push(Attribute::from_raw(
                    BuiltIn(Ident::new("id", Span::call_site())),
                    AttrLiteral(HotLiteral::from_raw_text(id)),
                ));
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
