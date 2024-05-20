//! An arbitrary block parser.
//!
//! Is meant to parse the contents of a block that is either a component or an element.
//! We put these together to cut down on code duplication and make the parsers a bit more resilient.
//!
//! This involves custom structs for name, attributes, and children, as well as a custom parser for the block itself.
//! It also bubbles out diagnostics if it can to give better errors.

use crate::{
    is_if_chain_terminated, location::CallerLocation, BodyNode, ElementAttrValue, IfmtInput,
};
use krates::cfg_expr::expr::lexer::Token;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseBuffer},
    spanned::Spanned,
    token::{self, At, Brace},
    AngleBracketedGenericArguments, Expr, ExprIf, Ident, Lit, LitStr, PatLit, PathArguments, Token,
};

/// An item in the form of
///
/// name<Generics> {
///  attributes,
///  children
/// }
///
/// Does not make any guarnatees about the contents of the block - this is meant to be verified by the
/// element/component impls themselves.
///
/// todo: add some diagnostics
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct RsxBlock {
    pub name: syn::Path,
    pub generics: Option<AngleBracketedGenericArguments>,
    pub fields: Vec<Attribute>,
    pub children: Vec<BodyNode>,
    pub brace: token::Brace,
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct Attribute {
    pub name: AttributeName,
    pub value: AttributeValue,
    pub dyn_idx: CallerLocation,
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum AttributeName {
    Custom(LitStr),
    Known(Ident),
    Spread(Token![..]),
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum AttributeValue {
    /// attribute,
    Shorthand(Ident),

    /// attribute: "value"
    AttrIfmt(IfmtInput),

    /// Unterminated expression - full expressions are handled by AttrExpr
    /// attribute: if bool { "value" }
    AttrOptionalExpr {
        condition: Expr,
        value: Box<AttributeValue>,
    },

    /// attribute: true
    AttrExpr(Expr),

    /// onclick: move |_| {}
    EventTokens(Expr),

    /// ..spread
    Spread(Expr),
}

impl Parse for RsxBlock {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // Parse the name of the block.
        // todo: add more partial parsing options to give better autocomplete
        let mut name = input.parse::<syn::Path>()?;

        // extract the path arguments from the path into prop_gen_args
        let generics = normalize_path(&mut name);

        let content: ParseBuffer;
        let brace = syn::braced!(content in input);

        // Parse attributes
        let mut attributes = vec![];
        loop {
            // If there's more than 1 attr and we're not at the end, we need a comma
            if attributes.len() > 0 && !content.is_empty() {
                content.parse::<Token![,]>()?; // <--- diagnostics...
            }

            if content.is_empty() {
                break;
            }

            // Parse spread attributes
            // For components this might actually be a props spread, which needs to come last
            if content.peek(Token![..]) {
                let _spread = content.parse::<Token![..]>()?;
                let expr = content.parse::<Expr>()?;
                attributes.push(Attribute {
                    name: AttributeName::Spread(_spread),
                    value: AttributeValue::Spread(expr),
                    dyn_idx: CallerLocation::default(),
                });
                continue;
            }

            // Parse shorthand attributes
            // todo: this might cause complications with partial expansion... think more about the cases
            // where we can imagine expansion and what better diagnostics we can providea
            if content.peek(Ident)
                && !content.peek2(Brace)
                && !content.peek2(Token![:])
                && !content.peek2(Token![-])
            {
                let name = content.parse::<Ident>()?;
                attributes.push(Attribute {
                    name: AttributeName::Known(name.clone()),
                    value: AttributeValue::Shorthand(name),
                    dyn_idx: CallerLocation::default(),
                });
                continue;
            }

            // We're going to just try parsing the next attribute directly, so early escape if it's not
            // in the right form
            if !((content.peek(LitStr) || content.peek(Ident))
                // And followed by a colon
                && content.peek2(Token![:]))
            {
                break;
            }

            // Parse the name as either a known or custom attribute
            let name = match content.peek(LitStr) {
                true => AttributeName::Custom(content.parse::<LitStr>()?),
                false => AttributeName::Known(content.parse::<Ident>()?),
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
                                    let value: Result<IfmtInput, syn::Error> =
                                        syn::parse2(quote! { #exp });

                                    match value {
                                        Ok(res) => Box::new(AttributeValue::AttrIfmt(res)),
                                        Err(_) => Box::new(AttributeValue::AttrExpr(exp.clone())),
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
            } else if content.peek(LitStr) {
                let value = content.parse()?;
                AttributeValue::AttrIfmt(value)
            } else if content.peek(Token![move]) || content.peek(Token![|]) {
                // todo: add better partial expansion
                let value = content.parse()?;
                AttributeValue::EventTokens(value)
            } else {
                let value = content.parse::<Expr>()?;
                AttributeValue::AttrExpr(value)
            };

            attributes.push(Attribute {
                name,
                value,
                dyn_idx: CallerLocation::default(),
            });
        }

        // Parse children
        let mut child_nodes = vec![];
        while !content.is_empty() {
            let child = content.parse()?;

            // try to give helpful diagnostic if a prop is in the wrong location

            child_nodes.push(child);
        }

        Ok(Self {
            name,
            generics,
            fields: attributes,
            children: child_nodes,
            brace,
        })
    }
}

impl RsxBlock {
    // peek the stream to see if this will parse as a block
    pub fn peek(input: syn::parse::ParseStream) -> bool {
        todo!()
    }
}

impl AttributeName {
    pub fn ident_to_str(&self) -> String {
        match self {
            Self::Custom(lit) => lit.value(),
            Self::Known(ident) => ident.to_string(),
            Self::Spread(_) => "..".to_string(),
        }
    }

    pub fn span(&self) -> proc_macro2::Span {
        match self {
            Self::Custom(lit) => lit.span(),
            Self::Known(ident) => ident.span(),
            Self::Spread(token) => token.span(),
        }
    }
}

impl AttributeValue {
    pub fn span(&self) -> proc_macro2::Span {
        match self {
            Self::Shorthand(ident) => ident.span(),
            Self::AttrIfmt(ifmt) => ifmt.span(),
            Self::AttrOptionalExpr { value, .. } => value.span(),
            Self::AttrExpr(expr) => expr.span(),
            Self::EventTokens(expr) => expr.span(),
            Self::Spread(expr) => expr.span(),
        }
    }
}

impl Attribute {
    pub fn span(&self) -> proc_macro2::Span {
        self.name.span()
    }

    pub fn ifmt(&self) -> Option<&IfmtInput> {
        match &self.value {
            AttributeValue::AttrIfmt(ifmt) => Some(ifmt),
            _ => None,
        }
    }
}

impl ToTokens for AttributeValue {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Self::Shorthand(ident) => ident.to_tokens(tokens),
            Self::AttrIfmt(ifmt) => ifmt.to_tokens(tokens),
            Self::AttrOptionalExpr { condition, value } => {
                tokens.append_all(quote! { if #condition { Some(#value) else { None } } })
            }
            Self::AttrExpr(expr) => expr.to_tokens(tokens),
            Self::EventTokens(expr) => expr.to_tokens(tokens),
            Self::Spread(expr) => expr.to_tokens(tokens),
        }
    }
}

fn normalize_path(name: &mut syn::Path) -> Option<AngleBracketedGenericArguments> {
    let seg = name.segments.last_mut()?;
    match seg.arguments.clone() {
        PathArguments::AngleBracketed(args) => {
            seg.arguments = PathArguments::None;
            Some(args)
        }
        _ => None,
    }
}

#[test]
fn basic_cases() {
    let input = quote! {
        div { "Hello, world!" }
    };

    let block: RsxBlock = syn::parse2(input).unwrap();
    assert_eq!(block.name, syn::parse_str("div").unwrap());
    assert_eq!(block.generics, None);
    assert_eq!(block.fields.len(), 0);
    assert_eq!(block.children.len(), 1);

    let input = quote! {
        Component<Generic> {
            key: "value",
            ..spread,
            onclick: move |_| {
                "Hello, world!"
            },
            "Hello, world!"
        }
    };

    let block: RsxBlock = syn::parse2(input).unwrap();
    dbg!(block);

    let complex_element = quote! {
        div {
            key: "value",
            ..spread,
            ..spread1,
            onclick2: move |_| {
                "Hello, world!"
            },
            ..spread2,
            thing: if true { "value" },
            otherthing: if true { "value" } else { "value" },
            onclick: move |_| {
                "Hello, world!"
            },
            "Hello, world!"
        }
    };

    let block: RsxBlock = syn::parse2(complex_element).unwrap();

    let complex_component = quote! {
        ::crate::some::other::Component<Generic> {
            key: "value",
            ..spread,
            onclick2: move |_| {
                "Hello, world!"
            },
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
        div {
            onclick: move |_| {
                some
            }
        }
    };

    let block: RsxBlock = syn::parse2(with_hander).unwrap();
}
