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
    pub captured_expressions: Vec<Expr>,
}

impl CapturedContextBuilder {
    pub fn extend(&mut self, other: CapturedContextBuilder) {
        self.attributes.extend(other.attributes);
        self.text.extend(other.text);
        self.components.extend(other.components);
        self.iterators.extend(other.iterators);
        self.captured_expressions.extend(other.captured_expressions);
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
                    match attr.attr {
                        ElementAttr::AttrText { name, value } => {
                            let (name, value_tokens) = (name.to_string(), value.to_token_stream());
                            let formated: IfmtInput = syn::parse2(value_tokens).unwrap();
                            captured.attributes.insert(name, formated);
                        }
                        ElementAttr::AttrExpression { name: _, value } => {
                            captured.captured_expressions.push(value);
                        }
                        ElementAttr::CustomAttrText { name, value } => {
                            let (name, value_tokens) = (name.value(), value.to_token_stream());
                            let formated: IfmtInput = syn::parse2(value_tokens).unwrap();
                            captured.attributes.insert(name, formated);
                        }
                        ElementAttr::CustomAttrExpression { name: _, value } => {
                            captured.captured_expressions.push(value);
                        }
                        _ => (),
                    }
                }
                for child in el.children {
                    captured.extend(Self::find_captured(child));
                }
            }
            BodyNode::Component(comp) => {
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
            captured_expressions,
        } = self;
        let compontents_str = components
            .iter()
            .map(|comp| comp.to_token_stream().to_string());
        let components = components.iter().map(|comp| comp);
        let iterators_str = iterators
            .iter()
            .map(|expr| expr.to_token_stream().to_string());
        let captured: Vec<_> = attributes
            .iter()
            .map(|(_, fmt)| fmt.named_args.iter())
            .chain(text.iter().map(|fmt| fmt.named_args.iter()))
            .flatten()
            .collect();
        let captured_names = captured.iter().map(|(n, _)| n.to_string());
        let captured_expr = captured.iter().map(|(_, e)| e);
        let captured_attr_expressions_text = captured_expressions
            .iter()
            .map(|e| format!("{}", e.to_token_stream()));
        tokens.append_all(quote! {
            CapturedContext {
                captured: IfmtArgs{
                    named_args: vec![#((#captured_names, #captured_expr.to_string())),*]
                },
                components: vec![#((#compontents_str, #components)),*],
                iterators: vec![#((#iterators_str, #iterators)),*],
                expressions: vec![#((#captured_attr_expressions_text, #captured_expressions.to_string())),*],
            }
        })
    }
}

pub struct CapturedContext<'a> {
    // map of the variable name to the formated value
    pub captured: IfmtArgs,
    // // map of the attribute name and element path to the formated value
    // pub captured_attribute_values: IfmtArgs,
    // the only thing we can update in component is the children
    pub components: Vec<(&'static str, VNode<'a>)>,
    // we can't reasonably interpert iterators, so they are staticly inserted
    pub iterators: Vec<(&'static str, VNode<'a>)>,
    // map expression to the value resulting from the expression
    pub expressions: Vec<(&'static str, String)>,
}

pub struct IfmtArgs {
    // live reload only supports named arguments
    pub named_args: Vec<(&'static str, String)>,
}
