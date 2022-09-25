use std::collections::HashSet;

use dioxus_core::{Listener, VNode};
use dioxus_rsx::{
    BodyNode, CallBody, Component, ElementAttr, ElementAttrNamed, IfmtInput, Segment,
};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{Expr, Ident, LitStr, Result};

use crate::{attributes::attrbute_to_static_str, CodeLocation};
#[derive(Default)]
pub struct CapturedContextBuilder {
    pub ifmted: Vec<IfmtInput>,
    pub components: Vec<Component>,
    pub iterators: Vec<BodyNode>,
    pub captured_expressions: Vec<Expr>,
    pub listeners: Vec<ElementAttrNamed>,
    pub custom_context: Option<Ident>,
    pub custom_attributes: HashSet<LitStr>,
}

impl CapturedContextBuilder {
    pub fn extend(&mut self, other: CapturedContextBuilder) {
        self.ifmted.extend(other.ifmted);
        self.components.extend(other.components);
        self.iterators.extend(other.iterators);
        self.listeners.extend(other.listeners);
        self.captured_expressions.extend(other.captured_expressions);
        self.custom_attributes.extend(other.custom_attributes);
    }

    pub fn from_call_body(body: CallBody) -> Result<Self> {
        let mut new = Self::default();
        for node in body.roots {
            new.extend(Self::find_captured(node)?);
        }
        Ok(new)
    }

    fn find_captured(node: BodyNode) -> Result<Self> {
        let mut captured = CapturedContextBuilder::default();
        match node {
            BodyNode::Element(el) => {
                for attr in el.attributes {
                    match attr.attr {
                        ElementAttr::AttrText { value, .. } => {
                            let value_tokens = value.to_token_stream();
                            let formated: IfmtInput = syn::parse2(value_tokens)?;
                            captured.ifmted.push(formated);
                        }
                        ElementAttr::CustomAttrText { value, name } => {
                            captured.custom_attributes.insert(name);
                            let value_tokens = value.to_token_stream();
                            let formated: IfmtInput = syn::parse2(value_tokens)?;
                            captured.ifmted.push(formated);
                        }
                        ElementAttr::AttrExpression { name: _, value } => {
                            captured.captured_expressions.push(value);
                        }
                        ElementAttr::CustomAttrExpression { name, value } => {
                            captured.custom_attributes.insert(name);
                            captured.captured_expressions.push(value);
                        }
                        ElementAttr::EventTokens { .. } => captured.listeners.push(attr),
                    }
                }

                if let Some(key) = el.key {
                    let value_tokens = key.to_token_stream();
                    let formated: IfmtInput = syn::parse2(value_tokens)?;
                    captured.ifmted.push(formated);
                }

                for child in el.children {
                    captured.extend(Self::find_captured(child)?);
                }
            }
            BodyNode::Component(comp) => {
                captured.components.push(comp);
            }
            BodyNode::Text(t) => {
                let tokens = t.to_token_stream();
                let formated: IfmtInput = syn::parse2(tokens).unwrap();
                captured.ifmted.push(formated);
            }
            BodyNode::RawExpr(_) => captured.iterators.push(node),
        }
        Ok(captured)
    }
}

impl ToTokens for CapturedContextBuilder {
    fn to_tokens(&self, tokens: &mut quote::__private::TokenStream) {
        let CapturedContextBuilder {
            ifmted,
            components,
            iterators,
            captured_expressions,
            listeners,
            custom_context: _,
            custom_attributes,
        } = self;
        let listeners_str = listeners
            .iter()
            .map(|comp| comp.to_token_stream().to_string());
        let compontents_str = components
            .iter()
            .map(|comp| comp.to_token_stream().to_string());
        let iterators_str = iterators.iter().map(|node| match node {
            BodyNode::RawExpr(expr) => expr.to_token_stream().to_string(),
            _ => unreachable!(),
        });
        let captured: Vec<_> = ifmted
            .iter()
            .flat_map(|input| input.segments.iter())
            .filter_map(|seg| match seg {
                Segment::Formatted {
                    format_args,
                    segment,
                } => {
                    let expr = segment.to_token_stream();
                    let as_string = expr.to_string();
                    let format_expr = if format_args.is_empty() {
                        "{".to_string() + format_args + "}"
                    } else {
                        "{".to_string() + ":" + format_args + "}"
                    };
                    Some(quote! {
                        FormattedArg{
                            expr: #as_string,
                            format_args: #format_args,
                            result: format!(#format_expr, #expr)
                        }
                    })
                }
                _ => None,
            })
            .collect();
        let captured_attr_expressions_text = captured_expressions
            .iter()
            .map(|e| format!("{}", e.to_token_stream()));
        let custom_attributes_iter = custom_attributes.iter();
        tokens.append_all(quote! {
            CapturedContext {
                captured: IfmtArgs{
                    named_args: vec![#(#captured),*]
                },
                components: vec![#((#compontents_str, #components)),*],
                iterators: vec![#((#iterators_str, #iterators)),*],
                expressions: vec![#((#captured_attr_expressions_text, #captured_expressions.to_string())),*],
                listeners: vec![#((#listeners_str, #listeners)),*],
                custom_attributes: &[#(#custom_attributes_iter),*],
                location: code_location.clone()
            }
        })
    }
}

pub struct CapturedContext<'a> {
    // map of the variable name to the formated value
    pub captured: IfmtArgs,
    // map of the attribute name and element path to the formated value
    // pub captured_attribute_values: IfmtArgs,
    // the only thing we can update in component is the children
    pub components: Vec<(&'static str, VNode<'a>)>,
    // we can't reasonably interpert iterators, so they are staticly inserted
    pub iterators: Vec<(&'static str, VNode<'a>)>,
    // map expression to the value resulting from the expression
    pub expressions: Vec<(&'static str, String)>,
    // map listener code to the resulting listener
    pub listeners: Vec<(&'static str, Listener<'a>)>,
    // used to map custom attrbutes form &'a str to &'static str
    pub custom_attributes: &'static [&'static str],
    // used to provide better error messages
    pub location: CodeLocation,
}

impl<'a> CapturedContext<'a> {
    pub fn attrbute_to_static_str(
        &self,
        attr: &str,
        tag: &'static str,
        ns: Option<&'static str>,
        literal: bool,
    ) -> Option<(&'static str, Option<&'static str>)> {
        if let Some(attr) = attrbute_to_static_str(attr, tag, ns) {
            Some(attr)
        } else if literal {
            self.custom_attributes
                .iter()
                .find(|attribute| attr == **attribute)
                .map(|attribute| (*attribute, None))
        } else {
            None
        }
    }
}

pub struct IfmtArgs {
    // All expressions that have been resolved
    pub named_args: Vec<FormattedArg>,
}

/// A formated segment that has been resolved
pub struct FormattedArg {
    pub expr: &'static str,
    pub format_args: &'static str,
    pub result: String,
}
