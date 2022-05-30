use dioxus_core::{Attribute, AttributeValue, NodeFactory, VNode};
use dioxus_rsx::{BodyNode, CallBody, ElementAttr, IfmtInput, Segment};
use quote::ToTokens;
use syn::{parse2, parse_str, Expr};

use crate::attributes::attrbute_to_static_str;
use crate::captuered_context::{CapturedContext, IfmtArgs};
use crate::elements::element_to_static_str;
use crate::error::{Error, RecompileReason};

struct InterpertedIfmt(IfmtInput);

impl InterpertedIfmt {
    fn resolve(&self, captured: &IfmtArgs) -> String {
        let mut result = String::new();
        for seg in &self.0.segments {
            match seg {
                Segment::Formatted {
                    segment,
                    format_args,
                } => {
                    let expr = segment.to_token_stream();
                    let expr_str = expr.to_string();
                    let expr: Expr = parse2(expr).unwrap();
                    let formatted = captured
                        .named_args
                        .iter()
                        .find(|fmted| {
                            parse_str::<Expr>(fmted.expr).unwrap() == expr
                                && fmted.format_args == format_args
                        })
                        .expect(
                            format!("could not resolve {{{}:{}}}", expr_str, format_args).as_str(),
                        );
                    result.push_str(&formatted.result);
                }
                Segment::Literal(lit) => result.push_str(lit),
            }
        }
        result
    }
}

pub fn build<'a>(
    rsx: CallBody,
    mut ctx: CapturedContext<'a>,
    factory: &NodeFactory<'a>,
) -> Result<VNode<'a>, Error> {
    let children_built = factory.bump().alloc(Vec::new());
    for child in rsx.roots {
        children_built.push(build_node(child, &mut ctx, factory)?);
    }
    Ok(factory.fragment_from_iter(children_built.iter()))
}

fn build_node<'a>(
    node: BodyNode,
    ctx: &mut CapturedContext<'a>,
    factory: &NodeFactory<'a>,
) -> Result<VNode<'a>, Error> {
    let bump = factory.bump();
    match node {
        BodyNode::Text(text) => {
            let ifmt = InterpertedIfmt(
                IfmtInput::from_str(&text.value()).map_err(|err| Error::ParseError(err))?,
            );
            let text = bump.alloc(ifmt.resolve(&ctx.captured));
            Ok(factory.text(format_args!("{}", text)))
        }
        BodyNode::Element(el) => {
            let attributes: &mut Vec<Attribute> = bump.alloc(Vec::new());
            for attr in &el.attributes {
                match &attr.attr {
                    ElementAttr::AttrText { .. } | ElementAttr::CustomAttrText { .. } => {
                        let (name, value): (String, InterpertedIfmt) = match &attr.attr {
                            ElementAttr::AttrText { name, value } => (
                                name.to_string(),
                                InterpertedIfmt(
                                    IfmtInput::from_str(&value.value())
                                        .map_err(|err| Error::ParseError(err))?,
                                ),
                            ),
                            ElementAttr::CustomAttrText { name, value } => (
                                name.value(),
                                InterpertedIfmt(
                                    IfmtInput::from_str(&value.value())
                                        .map_err(|err| Error::ParseError(err))?,
                                ),
                            ),
                            _ => unreachable!(),
                        };

                        if let Some((name, namespace)) = attrbute_to_static_str(&name) {
                            let value = bump.alloc(value.resolve(&ctx.captured));
                            attributes.push(Attribute {
                                name,
                                value: AttributeValue::Text(value),
                                is_static: true,
                                is_volatile: false,
                                namespace,
                            });
                        }
                    }

                    ElementAttr::AttrExpression { .. }
                    | ElementAttr::CustomAttrExpression { .. } => {
                        let (name, value) = match &attr.attr {
                            ElementAttr::AttrExpression { name, value } => {
                                (name.to_string(), value)
                            }
                            ElementAttr::CustomAttrExpression { name, value } => {
                                (name.value(), value)
                            }
                            _ => unreachable!(),
                        };
                        if let Some((_, resulting_value)) = ctx
                            .expressions
                            .iter()
                            .find(|(n, _)| parse_str::<Expr>(*n).unwrap() == *value)
                        {
                            if let Some((name, namespace)) = attrbute_to_static_str(&name) {
                                let value = bump.alloc(resulting_value.clone());
                                attributes.push(Attribute {
                                    name,
                                    value: AttributeValue::Text(value),
                                    is_static: true,
                                    is_volatile: false,
                                    namespace,
                                });
                            }
                        } else {
                            return Err(Error::RecompileRequiredError(
                                RecompileReason::CapturedExpression(
                                    value.into_token_stream().to_string(),
                                ),
                            ));
                        }
                    }
                    _ => (),
                };
            }
            let children = bump.alloc(Vec::new());
            for child in el.children {
                let node = build_node(child, ctx, factory);
                if let Ok(node) = node {
                    children.push(node);
                }
            }
            let listeners = bump.alloc(Vec::new());
            for attr in el.attributes {
                match attr.attr {
                    ElementAttr::EventTokens { .. } => {
                        let expr: Expr =
                            parse2(attr.to_token_stream()).map_err(|err| Error::ParseError(err))?;
                        if let Some(idx) = ctx.listeners.iter().position(|(code, _)| {
                            if let Ok(parsed) = parse_str::<Expr>(*code) {
                                parsed == expr
                            } else {
                                false
                            }
                        }) {
                            let (_, listener) = ctx.listeners.remove(idx);
                            listeners.push(listener)
                        } else {
                            return Err(Error::RecompileRequiredError(
                                RecompileReason::CapturedListener(
                                    expr.to_token_stream().to_string(),
                                ),
                            ));
                        }
                    }
                    _ => (),
                }
            }
            let tag = bump.alloc(el.name.to_string());
            if let Some((tag, ns)) = element_to_static_str(tag) {
                match el.key {
                    None => Ok(factory.raw_element(
                        tag,
                        ns,
                        listeners,
                        attributes.as_slice(),
                        children.as_slice(),
                        None,
                    )),
                    Some(lit) => {
                        let ifmt: InterpertedIfmt = InterpertedIfmt(
                            parse_str(&lit.value()).map_err(|err| Error::ParseError(err))?,
                        );
                        let key = bump.alloc(ifmt.resolve(&ctx.captured));

                        Ok(factory.raw_element(
                            tag,
                            ns,
                            listeners,
                            attributes.as_slice(),
                            children.as_slice(),
                            Some(format_args!("{}", key)),
                        ))
                    }
                }
            } else {
                Err(Error::ParseError(syn::Error::new(
                    el.name.span(),
                    "unknown element",
                )))
            }
        }
        BodyNode::Component(comp) => {
            let expr: Expr =
                parse2(comp.to_token_stream()).map_err(|err| Error::ParseError(err))?;
            if let Some(idx) = ctx.components.iter().position(|(code, _)| {
                if let Ok(parsed) = parse_str::<Expr>(*code) {
                    parsed == expr
                } else {
                    false
                }
            }) {
                let (_, vnode) = ctx.components.remove(idx);
                Ok(vnode)
            } else {
                Err(Error::RecompileRequiredError(
                    RecompileReason::CapturedComponent(comp.name.to_token_stream().to_string()),
                ))
            }
        }
        BodyNode::RawExpr(iterator) => {
            if let Some(idx) = ctx.iterators.iter().position(|(code, _)| {
                if let Ok(parsed) = parse_str::<Expr>(*code) {
                    parsed == iterator
                } else {
                    false
                }
            }) {
                let (_, vnode) = ctx.iterators.remove(idx);
                Ok(vnode)
            } else {
                Err(Error::RecompileRequiredError(
                    RecompileReason::CapturedExpression(iterator.to_token_stream().to_string()),
                ))
            }
        }
    }
}
