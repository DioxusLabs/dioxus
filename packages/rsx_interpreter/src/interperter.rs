use dioxus_core::{Attribute, AttributeValue, NodeFactory, VNode};
use dioxus_rsx::{BodyNode, CallBody, ElementAttr, IfmtInput, Segment};
use quote::ToTokens;
use quote::__private::Span;
use std::str::FromStr;
use syn::{parse2, parse_str, Expr};

use crate::captuered_context::{CapturedContext, IfmtArgs};
use crate::elements::element_to_static_str;
use crate::error::{Error, ParseError, RecompileReason};

fn resolve_ifmt(ifmt: &IfmtInput, captured: &IfmtArgs) -> Result<String, Error> {
    let mut result = String::new();
    for seg in &ifmt.segments {
        match seg {
            Segment::Formatted {
                segment,
                format_args,
            } => {
                let expr = segment.to_token_stream();
                let expr: Expr = parse2(expr).unwrap();
                let search = captured.named_args.iter().find(|fmted| {
                    parse_str::<Expr>(fmted.expr).unwrap() == expr
                        && fmted.format_args == format_args
                });
                match search {
                    Some(formatted) => {
                        result.push_str(&formatted.result);
                    }
                    None => {
                        let expr_str = segment.to_token_stream().to_string();
                        return Err(Error::RecompileRequiredError(
                            RecompileReason::CapturedExpression(format!(
                                "could not resolve {{{}:{}}}",
                                expr_str, format_args
                            )),
                        ));
                    }
                }
            }
            Segment::Literal(lit) => result.push_str(&lit),
        }
    }
    Ok(result)
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

    if children_built.len() == 1 {
        Ok(children_built.pop().unwrap())
    } else {
        Ok(factory.fragment_from_iter(children_built.iter()))
    }
}

fn build_node<'a>(
    node: BodyNode,
    ctx: &mut CapturedContext<'a>,
    factory: &NodeFactory<'a>,
) -> Result<VNode<'a>, Error> {
    let bump = factory.bump();
    match node {
        BodyNode::Text(text) => {
            let ifmt = IfmtInput::from_str(&text.value())
                .map_err(|err| Error::ParseError(ParseError::new(err, ctx.location.clone())))?;
            let text = bump.alloc(resolve_ifmt(&ifmt, &ctx.captured)?);
            Ok(factory.text(format_args!("{}", text)))
        }
        BodyNode::Element(el) => {
            let attributes: &mut Vec<Attribute> = bump.alloc(Vec::new());
            let tag = &el.name.to_string();
            if let Some((tag, ns)) = element_to_static_str(tag) {
                for attr in &el.attributes {
                    match &attr.attr {
                        ElementAttr::AttrText { .. } | ElementAttr::CustomAttrText { .. } => {
                            let (name, value, span, literal): (String, IfmtInput, Span, bool) =
                                match &attr.attr {
                                    ElementAttr::AttrText { name, value } => (
                                        name.to_string(),
                                        IfmtInput::from_str(&value.value()).map_err(|err| {
                                            Error::ParseError(ParseError::new(
                                                err,
                                                ctx.location.clone(),
                                            ))
                                        })?,
                                        name.span(),
                                        false,
                                    ),
                                    ElementAttr::CustomAttrText { name, value } => (
                                        name.value(),
                                        IfmtInput::from_str(&value.value()).map_err(|err| {
                                            Error::ParseError(ParseError::new(
                                                err,
                                                ctx.location.clone(),
                                            ))
                                        })?,
                                        name.span(),
                                        true,
                                    ),
                                    _ => unreachable!(),
                                };

                            if let Some((name, namespace)) =
                                ctx.attrbute_to_static_str(&name, tag, ns, literal)
                            {
                                let value = bump.alloc(resolve_ifmt(&value, &ctx.captured)?);
                                attributes.push(Attribute {
                                    name,
                                    value: AttributeValue::Text(value),
                                    is_static: true,
                                    is_volatile: false,
                                    namespace,
                                });
                            } else {
                                if literal {
                                    // literals will be captured when a full recompile is triggered
                                    return Err(Error::RecompileRequiredError(
                                        RecompileReason::CapturedAttribute(name.to_string()),
                                    ));
                                } else {
                                    return Err(Error::ParseError(ParseError::new(
                                        syn::Error::new(
                                            span,
                                            format!("unknown attribute: {}", name),
                                        ),
                                        ctx.location.clone(),
                                    )));
                                }
                            }
                        }

                        ElementAttr::AttrExpression { .. }
                        | ElementAttr::CustomAttrExpression { .. } => {
                            let (name, value, span, literal) = match &attr.attr {
                                ElementAttr::AttrExpression { name, value } => {
                                    (name.to_string(), value, name.span(), false)
                                }
                                ElementAttr::CustomAttrExpression { name, value } => {
                                    (name.value(), value, name.span(), true)
                                }
                                _ => unreachable!(),
                            };
                            if let Some((_, resulting_value)) = ctx
                                .expressions
                                .iter()
                                .find(|(n, _)| parse_str::<Expr>(*n).unwrap() == *value)
                            {
                                if let Some((name, namespace)) =
                                    ctx.attrbute_to_static_str(&name, tag, ns, literal)
                                {
                                    let value = bump.alloc(resulting_value.clone());
                                    attributes.push(Attribute {
                                        name,
                                        value: AttributeValue::Text(value),
                                        is_static: true,
                                        is_volatile: false,
                                        namespace,
                                    });
                                } else {
                                    if literal {
                                        // literals will be captured when a full recompile is triggered
                                        return Err(Error::RecompileRequiredError(
                                            RecompileReason::CapturedAttribute(name.to_string()),
                                        ));
                                    } else {
                                        return Err(Error::ParseError(ParseError::new(
                                            syn::Error::new(
                                                span,
                                                format!("unknown attribute: {}", name),
                                            ),
                                            ctx.location.clone(),
                                        )));
                                    }
                                }
                            }
                        }
                        _ => (),
                    }
                }
                let children = bump.alloc(Vec::new());
                for child in el.children {
                    let node = build_node(child, ctx, factory)?;
                    children.push(node);
                }
                let listeners = bump.alloc(Vec::new());
                for attr in el.attributes {
                    if let ElementAttr::EventTokens { .. } = attr.attr {
                        let expr: Expr = parse2(attr.to_token_stream()).map_err(|err| {
                            Error::ParseError(ParseError::new(err, ctx.location.clone()))
                        })?;
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
                }
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
                        let ifmt: IfmtInput = lit.value().parse().map_err(|err| {
                            Error::ParseError(ParseError::new(err, ctx.location.clone()))
                        })?;
                        let key = bump.alloc(resolve_ifmt(&ifmt, &ctx.captured)?);

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
                Err(Error::ParseError(ParseError::new(
                    syn::Error::new(el.name.span(), format!("unknown element: {}", tag)),
                    ctx.location.clone(),
                )))
            }
        }
        BodyNode::Component(comp) => {
            let expr: Expr = parse2(comp.to_token_stream())
                .map_err(|err| Error::ParseError(ParseError::new(err, ctx.location.clone())))?;
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
