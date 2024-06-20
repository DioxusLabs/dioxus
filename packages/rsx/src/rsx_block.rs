//! An arbitrary block parser.
//!
//! Is meant to parse the contents of a block that is either a component or an element.
//! We put these together to cut down on code duplication and make the parsers a bit more resilient.
//!
//! This involves custom structs for name, attributes, and children, as well as a custom parser for the block itself.
//! It also bubbles out diagnostics if it can to give better errors.

use std::fmt::Display;

use crate::{innerlude::*, HotReloadingContext};
use dioxus_core::prelude::TemplateAttribute;
use proc_macro2::{Literal, TokenStream};
use proc_macro2_diagnostics::SpanDiagnosticExt;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseBuffer},
    spanned::Spanned,
    token::{self, Brace},
    AngleBracketedGenericArguments, Expr, ExprClosure, ExprIf, Ident, Lit, LitStr, PatLit,
    PathArguments, Token,
};

use super::literal::RsxLiteral;

/// An item in the form of
///
/// {
///  attributes,
///  ..spreads,
///  children
/// }
///
/// Does not make any guarnatees about the contents of the block - this is meant to be verified by the
/// element/component impls themselves.
///
/// The name of the block is expected to be parsed by the parent parser. It will accept items out of
/// order if possible and then bubble up diagnostics to the parent. This lets us give better errors
/// and autocomplete
///
/// todo: add some diagnostics
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct RsxBlock {
    pub fields: Vec<Attribute>,
    pub spreads: Vec<Spread>,
    pub children: Vec<BodyNode>,
    pub brace: token::Brace,
    pub diagnostics: Diagnostics,
}

// ..spread attribute
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct Spread {
    pub expr: Expr,
    pub dyn_idx: DynIdx,
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum AttributeValue {
    /// Just a regular shorthand attribute - an ident. Makes our parsing a bit more opaque.
    /// attribute,
    Shorthand(Ident),

    /// Any attribute that's a literal. These get hotreloading super powers
    ///
    /// attribute: "value"
    /// attribute: bool,
    /// attribute: 1,
    AttrLiteral(RsxLiteral),

    /// Unterminated expression - full expressions are handled by AttrExpr
    ///
    /// attribute: if bool { "value" }
    ///
    /// Currently these don't get hotreloading super powers, but they could, depending on how far
    /// we want to go with it
    AttrOptionalExpr {
        condition: Expr,
        value: Box<AttributeValue>,
    },

    /// attribute: some_expr
    AttrExpr(Expr),
}

impl Parse for RsxBlock {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content: ParseBuffer;
        let brace = syn::braced!(content in input);

        // todo: toss a warning for
        // Parse attributes
        let mut attributes = vec![];
        let mut spreads = vec![];
        let mut diagnostics = Diagnostics::new();
        loop {
            if content.is_empty() {
                break;
            }

            // Parse spread attributes
            // These are expected forced to come after regular attributes
            if content.peek(Token![..]) {
                let _spread = content.parse::<Token![..]>()?;

                if content.peek(Token![.]) {
                    let _extra = content.parse::<Token![.]>()?;
                    diagnostics.push(
                        _extra
                            .span()
                            .error("Spread expressions only take two dots - not 3! (..spread)"),
                    );
                }

                let expr = content.parse::<Expr>()?;
                spreads.push(Spread {
                    expr,
                    dyn_idx: DynIdx::default(),
                });

                if !content.is_empty() {
                    content.parse::<Token![,]>()?; // <--- diagnostics...
                }
                continue;
            }

            // Parse shorthand attributes
            // todo: this might cause complications with partial expansion... think more about the cases
            // where we can imagine expansion and what better diagnostics we can providea
            if content.peek(Ident)
                && !content.peek2(Brace)
                && !content.peek2(Token![:])
                && !content.peek2(Token![-])
                && !content.peek2(token::Brace)
            {
                let name = content.parse::<Ident>()?;

                if !spreads.is_empty() {
                    diagnostics.push(name.span().error(
                        "Spread attributes must come after regular attributes and before children",
                    ));
                    diagnostics.push(spreads.last().unwrap().expr.span().warning(
                        "This spread attribute should be moved to the end of the attribute list",
                    ));
                }

                attributes.push(Attribute {
                    name: AttributeName::BuiltIn(name.clone()),
                    value: AttributeValue::Shorthand(name),
                    dyn_idx: DynIdx::default(),
                });

                if !content.is_empty() {
                    content.parse::<Token![,]>()?; // <--- diagnostics...
                }

                continue;
            }

            // Parse regular attributes
            if (content.peek(LitStr) || content.peek(Ident))
                && content.peek2(Token![:])
                && !content.peek3(Token![:])
            {
                // Parse the name as either a known or custom attribute
                let name = match content.peek(LitStr) {
                    true => AttributeName::Custom(content.parse::<LitStr>()?),
                    false => AttributeName::BuiltIn(content.parse::<Ident>()?),
                };

                // Ensure there's a colon
                _ = content.parse::<Token![:]>()?;

                // if statements in attributes get automatic closing in some cases
                let value = if content.peek(Token![if]) {
                    let if_expr = content.parse::<ExprIf>()?;
                    if is_if_chain_terminated(&if_expr) {
                        AttributeValue::AttrExpr(Expr::If(if_expr))
                    } else {
                        AttributeValue::AttrOptionalExpr {
                            condition: *if_expr.cond,
                            value: {
                                let stmts = &if_expr.then_branch.stmts;

                                if stmts.len() != 1 {
                                    return Err(syn::Error::new(
                                        if_expr.then_branch.span(),
                                        "Expected a single statement in the if block",
                                    ));
                                }

                                // either an ifmt or an expr in the block
                                let stmt = &stmts[0];

                                // Either it's a valid ifmt or an expression
                                match stmt {
                                    syn::Stmt::Expr(exp, None) => {
                                        // Try parsing the statement as an IfmtInput by passing it through tokens
                                        let value: Result<RsxLiteral, syn::Error> =
                                            syn::parse2(quote! { #exp });

                                        match value {
                                            Ok(res) => Box::new(AttributeValue::AttrLiteral(res)),
                                            Err(_) => {
                                                Box::new(AttributeValue::AttrExpr(exp.clone()))
                                            }
                                        }
                                    }
                                    _ => {
                                        return Err(syn::Error::new(
                                            stmt.span(),
                                            "Expected an expression",
                                        ))
                                    }
                                }
                            },
                        }
                    }
                } else if RsxLiteral::peek(&content) {
                    let value = content.parse()?;
                    AttributeValue::AttrLiteral(value)
                } else if content.peek(Token![move]) || content.peek(Token![|]) {
                    // todo: add better partial expansion for closures - that's why we're handling them differently here
                    let value: Expr = content.parse()?;
                    AttributeValue::AttrExpr(value)
                } else {
                    let value = content.parse::<Expr>()?;
                    AttributeValue::AttrExpr(value)
                };

                if !spreads.is_empty() {
                    diagnostics.push(name.span().error(
                        "Spread attributes must come after regular attributes and before children",
                    ));
                    diagnostics.push(spreads.last().unwrap().expr.span().warning(
                        "This spread attribute should be moved to the end of the attribute list",
                    ));
                }

                attributes.push(Attribute {
                    name,
                    value,
                    dyn_idx: DynIdx::default(),
                });

                if !content.is_empty() {
                    content.parse::<Token![,]>()?; // <--- diagnostics...
                }

                continue;
            }

            break;
        }

        // Parse children
        let mut child_nodes = vec![];
        while !content.is_empty() {
            let child: BodyNode = content.parse()?;

            // todo: try to give helpful diagnostic if a prop is in the wrong location
            child_nodes.push(child);

            if content.peek(Token![,]) {
                _ = content.parse::<Token![,]>()?;
            }
        }

        Ok(Self {
            fields: attributes,
            children: child_nodes,
            spreads,
            brace,
            diagnostics,
        })
    }
}

#[test]
fn basic_cases() {
    let input = quote! {
        { "Hello, world!" }
    };

    let block: RsxBlock = syn::parse2(input).unwrap();
    assert_eq!(block.fields.len(), 0);
    assert_eq!(block.children.len(), 1);

    let input = quote! {
        {
            key: "value",
            onclick: move |_| {
                "Hello, world!"
            },
            ..spread,
            "Hello, world!"
        }
    };

    let block: RsxBlock = syn::parse2(input).unwrap();
    dbg!(block);

    let complex_element = quote! {
        {
            key: "value",
            onclick2: move |_| {
                "Hello, world!"
            },
            thing: if true { "value" },
            otherthing: if true { "value" } else { "value" },
            onclick: move |_| {
                "Hello, world!"
            },
            ..spread,
            ..spread1
            ..spread2,
            "Hello, world!"
        }
    };

    let block: RsxBlock = syn::parse2(complex_element).unwrap();

    let complex_component = quote! {
        {
            key: "value",
            onclick2: move |_| {
                "Hello, world!"
            },
            ..spread,
            "Hello, world!"
        }
    };

    let block: RsxBlock = syn::parse2(complex_component).unwrap();
}

#[test]
fn ensure_props_before_elements() {}

/// Some tests of partial expansion to give better autocomplete
#[test]
fn partial_cases() {
    let with_hander = quote! {
        {
            onclick: move |_| {
                some
            }
        }
    };

    let block: RsxBlock = syn::parse2(with_hander).unwrap();
}

/// Give helpful errors in the cases where the tree is malformed but we can still give a good error
/// Usually this just boils down to incorrect orders
#[test]
fn proper_diagnostics() {}

/// Ensure the hotreload scoring algorithm works as expected
#[test]
fn hr_score() {
    let block = quote! {
        {
            a: "value {cool}",
            b: "{cool} value",
            b: "{cool} {thing} value",
            b: "{thing} value",
        }
    };

    // loop { accumulate perfect matches }
    // stop when all matches are equally valid
    //
    // Remove new attr one by one as we find its perfect match. If it doesn't have a perfect match, we
    // score it instead.

    quote! {
        // start with
        div {
            div { class: "other {abc} {def} {hij}" } // 1, 1, 1
            div { class: "thing {abc} {def}" }       // 1, 1, 1
            // div { class: "thing {abc}" }             // 1, 0, 1
        }

        // end with
        div {
            h1 {
                class: "thing {abc}" // 1, 1, MAX
            }
            h1 {
                class: "thing {hij}" // 1, 1, MAX
            }
            // h2 {
            //     class: "thing {def}" // 1, 1, 0
            // }
            // h3 {
            //     class: "thing {def}" // 1, 1, 0
            // }
        }

        // how about shuffling components, for, if, etc
        Component {
            class: "thing {abc}",
            other: "other {abc} {def}",
        }
        Component {
            class: "thing",
            other: "other",
        }

        Component {
            class: "thing {abc}",
            other: "other",
        }
        Component {
            class: "thing {abc}",
            other: "other {abc} {def}",
        }
    };
}

#[test]
fn kitchen_sink_parse() {
    let input = quote! {
        // Elements
        {
            class: "hello",
            id: "node-{node_id}",
            ..props,

            // Text Nodes
            "Hello, world!"

            // Exprs
            {rsx! { "hi again!" }}


            for item in 0..10 {
                // "Second"
                div { "cool-{item}" }
            }

            Link {
                to: "/home",
                class: "link {is_ready}",
                "Home"
            }

            if false {
                div { "hi again!?" }
            } else if true {
                div { "its cool?" }
            } else {
                div { "not nice !" }
            }
        }
    };

    let parsed: RsxBlock = syn::parse2(input).unwrap();
    // let tokens = quote! { #parsed };
    // println!("{}", tokens);
}

#[test]
fn simple_comp_syntax() {
    let input = quote! {
        { class: "inline-block mr-4", icons::icon_14 {} }
    };

    let parsed: RsxBlock = syn::parse2(input).unwrap();
}
