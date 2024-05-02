use dioxus_rsx::{
    AttributeType, BodyNode, CallBody, ElementAttrNamed, ElementAttrValue, ElementName,
};
use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Declaration, Expression, Function, JSXAttributeItem, JSXAttributeName, JSXAttributeValue,
    JSXChild, JSXElement, JSXElementName, JSXExpression, MemberExpression, ReturnStatement,
    Statement,
};
use oxc_parser::Parser;
use oxc_parser::*;
use oxc_span::SourceType;
use proc_macro2::Span;
use syn::{Ident, Macro, PatLit};

use crate::ifmt_from_text;

fn convert_program_to_rsx(ast: &oxc_ast::ast::Program) -> Option<dioxus_rsx::CallBody> {
    let Statement::Declaration(Declaration::FunctionDeclaration(function)) = &ast.body[0] else {
        eprintln!("Expected a function declaration at the beginning of the program.");
        return None;
    };

    let Statement::ReturnStatement(block) =
        &function.body.as_ref().unwrap().statements.last().unwrap()
    else {
        eprintln!("Expected a return statement at the end of the program.");
        return None;
    };

    let Some(Expression::JSXElement(el)) = block.argument.as_ref() else {
        eprintln!("Expected a JSXElement as the return value.");
        return None;
    };

    convret_jsx_element_to_rsx(el)
}

fn parse_as_bodynode(child: &JSXChild) -> Option<BodyNode> {
    let el = match child {
        JSXChild::Element(el) => {
            //
            let el_name = match &el.opening_element.name {
                JSXElementName::Identifier(ident) => {
                    let name = &ident.name;
                    name.to_string()
                }
                _ => todo!(),
            };

            let el_name_ident = ElementName::Ident(Ident::new(el_name.as_str(), Span::call_site()));

            let mut children = vec![];

            for child in &el.children {
                if let Some(child) = parse_as_bodynode(child) {
                    children.push(child);
                }
            }

            let mut attributes: Vec<AttributeType> = vec![];

            for attr in &el.opening_element.attributes {
                let value = match attr {
                    JSXAttributeItem::Attribute(attr) => {
                        let value: ElementAttrValue = match &attr.value {
                            Some(value) => match value {
                                JSXAttributeValue::StringLiteral(lit) => {
                                    ElementAttrValue::AttrLiteral(ifmt_from_text(&lit.value))
                                }

                                JSXAttributeValue::ExpressionContainer(expr) => {
                                    // todo: translate this JS to rust
                                    let JSXExpression::Expression(expression) = &expr.expression
                                    else {
                                        return None;
                                    };

                                    // match expression {
                                    //     Expression::BooleanLiteral(_) => todo!(),
                                    //     Expression::NullLiteral(_) => todo!(),
                                    //     Expression::NumericLiteral(_) => todo!(),
                                    //     Expression::BigintLiteral(_) => todo!(),
                                    //     Expression::RegExpLiteral(_) => todo!(),
                                    //     Expression::StringLiteral(_) => todo!(),
                                    //     Expression::TemplateLiteral(_) => todo!(),
                                    //     Expression::Identifier(_) => todo!(),
                                    //     Expression::MetaProperty(_) => todo!(),
                                    //     Expression::Super(_) => todo!(),
                                    //     Expression::ArrayExpression(_) => todo!(),
                                    //     Expression::ArrowFunctionExpression(_) => todo!(),
                                    //     Expression::AssignmentExpression(_) => todo!(),
                                    //     Expression::AwaitExpression(_) => todo!(),
                                    //     Expression::BinaryExpression(_) => todo!(),
                                    //     Expression::CallExpression(_) => todo!(),
                                    //     Expression::ChainExpression(_) => todo!(),
                                    //     Expression::ClassExpression(_) => todo!(),
                                    //     Expression::ConditionalExpression(_) => todo!(),
                                    //     Expression::FunctionExpression(_) => todo!(),
                                    //     Expression::ImportExpression(_) => todo!(),
                                    //     Expression::LogicalExpression(_) => todo!(),
                                    //     Expression::MemberExpression(member_expr) => {
                                    //         convert_member_expr_to_rust_expr(member_expr)
                                    //     }
                                    //     Expression::NewExpression(_) => todo!(),
                                    //     Expression::ObjectExpression(_) => todo!(),
                                    //     Expression::ParenthesizedExpression(_) => todo!(),
                                    //     Expression::SequenceExpression(_) => todo!(),
                                    //     Expression::TaggedTemplateExpression(_) => todo!(),
                                    //     Expression::ThisExpression(_) => todo!(),
                                    //     Expression::UnaryExpression(_) => todo!(),
                                    //     Expression::UpdateExpression(_) => todo!(),
                                    //     Expression::YieldExpression(_) => todo!(),
                                    //     Expression::PrivateInExpression(_) => todo!(),
                                    //     Expression::JSXElement(_) => todo!(),
                                    //     Expression::JSXFragment(_) => todo!(),
                                    //     Expression::TSAsExpression(_) => todo!(),
                                    //     Expression::TSSatisfiesExpression(_) => todo!(),
                                    //     Expression::TSTypeAssertion(_) => todo!(),
                                    //     Expression::TSNonNullExpression(_) => todo!(),
                                    //     Expression::TSInstantiationExpression(_) => todo!(),
                                    // }

                                    println!("{:#?}", expression);

                                    // todo!()

                                    let mac = quote::quote! {
                                        todo!()
                                    };

                                    let mac: Macro = syn::parse2(mac).unwrap();
                                    ElementAttrValue::AttrExpr(syn::Expr::Macro(syn::ExprMacro {
                                        attrs: vec![],
                                        // just write a todo!() here
                                        mac,
                                    }))
                                }
                                JSXAttributeValue::Element(_) => todo!(),
                                JSXAttributeValue::Fragment(_) => todo!(),
                            },
                            None => ElementAttrValue::AttrExpr(syn::Expr::Lit(syn::ExprLit {
                                attrs: vec![],
                                lit: syn::Lit::Bool(syn::LitBool {
                                    span: Span::call_site(),
                                    value: true,
                                }),
                            })),
                        };

                        let name = match &attr.name {
                            JSXAttributeName::Identifier(ident) => ident.name.to_string(),
                            JSXAttributeName::NamespacedName(_) => todo!(),
                        };

                        attributes.push(AttributeType::Named(ElementAttrNamed {
                            el_name: el_name_ident.clone(),
                            attr: dioxus_rsx::ElementAttr {
                                name: dioxus_rsx::ElementAttrName::BuiltIn(Ident::new(
                                    name.as_str(),
                                    Span::call_site(),
                                )),
                                value,
                            },
                        }));
                    }
                    JSXAttributeItem::SpreadAttribute(_) => todo!(),
                };

                // let name = attr.name.sym.to_string();
                // let value = match &attr.value {
                //     JSXAttrValue::Literal(lit) => lit.value.to_string(),
                //     JSXAttrValue::JSXExpressionContainer(expr) => {}
                // };
            }

            dioxus_rsx::BodyNode::Element(dioxus_rsx::Element {
                name: el_name_ident,
                children,
                attributes,
                merged_attributes: Default::default(),
                key: None,
                brace: Default::default(),
            })
        }
        JSXChild::Text(text) => {
            //
            let trimmed = text.value.as_str().trim();
            if trimmed.is_empty() {
                return None;
            }

            dioxus_rsx::BodyNode::Text(ifmt_from_text(trimmed))
        }
        JSXChild::Fragment(_) => return None,
        JSXChild::ExpressionContainer(_) => return None,
        JSXChild::Spread(_) => return None,
    };

    Some(el)
}

// convert "props.menu.icon" to "props.menu.icon"
fn convert_member_expr_to_rust_expr(member_expr: &MemberExpression<'_>) {
    let expr = match &*member_expr {
        MemberExpression::ComputedMemberExpression(_) => todo!(),
        MemberExpression::StaticMemberExpression(ref expr) => expr,
        MemberExpression::PrivateFieldExpression(_) => todo!(),
    };

    // let syn_expr = match expr.object {
    //     Expression::Identifier(ident) => {
    // let name = ident.name;
    // syn::Expr::Path(syn::ExprPath {
    //     attrs: vec![],
    //     qself: None,
    //     path: syn::Path {
    //         leading_colon: None,
    //         segments: vec![syn::PathSegment {
    //             ident: syn::Ident::new(name, Span::call_site()),
    //             arguments: syn::PathArguments::None,
    //         }],
    //     },
    // })
    // }
    // Expression::MemberExpression(member_expr) => {
    // let expr = convert_member_expr_to_rust_expr(member_expr);
    // syn::Expr::Path(syn::ExprPath {
    //     attrs: vec![],
    //     qself: None,
    //     path: syn::Path {
    //         leading_colon: None,
    //         segments: vec![syn::PathSegment {
    //             ident: syn::Ident::new(name, Span::call_site()),
    //             arguments: syn::PathArguments::None,
    //         }],
    //     },
    // })
    //     }
    //     _ => todo!(),
    // };
}

fn convret_jsx_element_to_rsx(el: &JSXElement) -> Option<dioxus_rsx::CallBody> {
    let root = match &el.opening_element.name {
        JSXElementName::Identifier(ident) => {
            let name = &ident.name;
            // let mut attributes = vec![];

            for attr in &el.opening_element.attributes {
                // let name = attr.name.sym.to_string();
                // let value = match &attr.value {
                //     JSXAttrValue::Literal(lit) => lit.value.to_string(),
                //     JSXAttrValue::JSXExpressionContainer(expr) => {}
                // };
            }

            let mut children = vec![];

            for child in &el.children {
                if let Some(child) = parse_as_bodynode(child) {
                    children.push(child);
                }
            }

            let name = ElementName::Ident(Ident::new(name, Span::call_site()));

            dioxus_rsx::BodyNode::Element(dioxus_rsx::Element {
                name,
                children,
                attributes: vec![],
                merged_attributes: Default::default(),
                key: None,
                brace: Default::default(),
            })
        }
        _ => todo!(),
    };

    Some(dioxus_rsx::CallBody { roots: vec![root] })
}

#[test]
fn it_parser() {
    let path = "tests/fixtures/demo.jsx";
    let source_text = std::fs::read_to_string(path).unwrap();

    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path).unwrap();
    let ret = Parser::new(&allocator, &source_text, source_type).parse();

    // println!("{}", serde_json::to_string_pretty(&ret.program).unwrap());

    let body: CallBody = convert_program_to_rsx(&ret.program).unwrap();

    let out = dioxus_autofmt::write_block_out(body).unwrap();

    println!("{}", out);

    // println!("AST:");

    // println!("Comments:");
    // let comments = ret
    //     .trivias
    //     .comments()
    //     .map(|(_, span)| span.source_text(&source_text))
    //     .collect::<Vec<_>>();
    // println!("{comments:?}");

    // if ret.errors.is_empty() {
    //     println!("Parsed Successfully.");
    // } else {
    //     for error in ret.errors {
    //         let error = error.with_source_code(source_text.clone());
    //         println!("{error:?}");
    //         println!("Parsed with Errors.");
    //     }
    // }
}

pub fn jsx_to_rsx() {}
