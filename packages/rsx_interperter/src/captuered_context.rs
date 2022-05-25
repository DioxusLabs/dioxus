use dioxus_core::VNode;
use dioxus_rsx::{BodyNode, CallBody, Component, ElementAttr, IfmtInput};
use quote::{quote, ToTokens, TokenStreamExt};
use std::collections::HashMap;
use syn::Expr;

#[derive(Default)]
pub struct CapturedContextBuilder {
    pub attributes: HashMap<String, IfmtInput>,
    pub text: Vec<IfmtInput>,
    pub components: Vec<Component>,
    pub iterators: Vec<Expr>,
}

impl CapturedContextBuilder {
    pub fn extend(&mut self, other: CapturedContextBuilder) {
        self.attributes.extend(other.attributes);
        self.text.extend(other.text);
        self.components.extend(other.components);
    }

    pub fn from_call_body(body: CallBody) -> Self {
        let mut new = Self::default();
        for node in body.roots {
            new.extend(Self::find_captured(node));
        }
        new
    }

    fn find_captured(node: BodyNode) -> Self {
        let mut captured = CapturedContextBuilder::default();
        match node {
            BodyNode::Element(el) => {
                for attr in el.attributes {
                    let (name, value_tokens) = match attr.attr {
                        ElementAttr::AttrText { name, value } => {
                            (name.to_string(), value.to_token_stream())
                        }
                        ElementAttr::AttrExpression { name, value } => {
                            (name.to_string(), value.to_token_stream())
                        }
                        ElementAttr::CustomAttrText { name, value } => {
                            (name.value(), value.to_token_stream())
                        }
                        ElementAttr::CustomAttrExpression { name, value } => {
                            (name.value(), value.to_token_stream())
                        }
                        _ => continue,
                    };
                    let formated: IfmtInput = syn::parse2(value_tokens).unwrap();
                    captured.attributes.insert(name, formated);
                }
                for child in el.children {
                    captured.extend(Self::find_captured(child));
                }
            }
            BodyNode::Component(comp) => {
                let fn_name = comp.name.segments.last().unwrap().ident.to_string();
                captured.components.push(comp);
            }
            BodyNode::Text(t) => {
                let tokens = t.to_token_stream();
                let formated: IfmtInput = syn::parse2(tokens).unwrap();
                captured.text.push(formated);
            }
            BodyNode::RawExpr(expr) => captured.iterators.push(expr),
            BodyNode::Meta(_) => (),
        }
        captured
    }
}

impl ToTokens for CapturedContextBuilder {
    fn to_tokens(&self, tokens: &mut quote::__private::TokenStream) {
        let CapturedContextBuilder {
            attributes,
            text,
            components,
            iterators,
        } = self;
        let captured: Vec<_> = attributes
            .iter()
            .map(|(_, fmt)| fmt.named_args.iter())
            .chain(text.iter().map(|fmt| fmt.named_args.iter()))
            .flatten()
            .collect();
        let captured_names = captured.iter().map(|(n, _)| n);
        let captured_expr = captured.iter().map(|(_, e)| e);
        tokens.append_all(quote! {
            CapturedContext {
                captured: IfmtArgs{
                    named_args: &[#((#captured_names, #captured_expr)),*]
                },
                components: vec![#(#components),*],
                iterators: vec![#(#iterators),*],
            }
        })
    }
}

struct CapturedComponentBuilder {
    name: syn::Path,
    function: String,
}

pub struct CapturedContext<'a> {
    // map of the attribute name to the formated value
    pub captured: IfmtArgs,
    // the only thing we can update in component is the children
    pub components: Vec<VNode<'a>>,
    // we can't reasonably interpert iterators, so they are staticly inserted
    pub iterators: Vec<VNode<'a>>,
}

pub struct IfmtArgs {
    // live reload only supports named arguments
    pub named_args: &'static [(&'static str, String)],
}

enum IfmtSegment<'a> {
    Static(&'a str),
    Dynamic(&'a str),
}

enum RsxNode<'a> {
    Element {
        name: String,
        attributes: Vec<(String, IfmtSegment<'a>)>,
        children: Vec<RsxNode<'a>>,
    },
    Text {
        text: Vec<IfmtSegment<'a>>,
    },
    Component {
        children: Vec<RsxNode<'a>>,
    },
}
